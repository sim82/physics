use bevy::{
    app::{AppExit, PluginGroupBuilder},
    diagnostic::FrameTimeDiagnosticsPlugin,
    prelude::*,
};
#[cfg(feature = "inspector")]
use bevy_inspector_egui::WorldInspectorParams;

use bevy_rapier3d::prelude::*;

pub mod contact_debug;
// pub mod debug_lines;
pub mod appearance;
pub mod editor;
pub mod sky;
pub mod slidemove;
pub mod trace;
pub mod wsx;

pub mod debug_gui;
pub mod norm {
    // srgb workaround from https://github.com/bevyengine/bevy/issues/6371
    use bevy::asset::{AssetLoader, Error, LoadContext, LoadedAsset};
    use bevy::render::texture::{CompressedImageFormats, Image, ImageType};
    use bevy::utils::BoxedFuture;

    #[derive(Default)]
    pub struct NormalMappedImageTextureLoader;

    impl AssetLoader for NormalMappedImageTextureLoader {
        fn load<'a>(
            &'a self,
            bytes: &'a [u8],
            load_context: &'a mut LoadContext,
        ) -> BoxedFuture<'a, Result<(), Error>> {
            Box::pin(async move {
                let dyn_img = Image::from_buffer(
                    bytes,
                    ImageType::Extension("png"),
                    CompressedImageFormats::all(),
                    false,
                )
                .unwrap();

                load_context.set_default_asset(LoadedAsset::new(dyn_img));
                Ok(())
            })
        }

        fn extensions(&self) -> &[&str] {
            &["norm"]
        }
    }
}

pub mod material;

pub mod render_layers {
    use bevy::render::view::{Layer, RenderLayers};

    pub const MAIN_3D: Layer = 0;
    pub const TOP_2D: Layer = 1;
    pub const SIDE_2D: Layer = 2;
    pub fn ortho_views() -> RenderLayers {
        RenderLayers::from_layers(&[TOP_2D, SIDE_2D])
    }
}

pub const OVERCLIP: f32 = 1.001;

pub mod test_texture {
    pub const TW: usize = 256;
    pub const TH: usize = 256;

    pub fn create() -> Vec<u8> {
        // let mut bitmap = [0u32; TW * TH];

        let mut bitmap = Vec::new();

        for y in 0..TH as i32 {
            for x in 0..TW as i32 {
                let l = 0x1FF
                    >> [x, y, TW as i32 - 1 - x, TH as i32 - 1 - y, 31]
                        .iter()
                        .min()
                        .unwrap();

                let d = (255
                    - 50 * f32::powf(
                        f32::hypot(
                            x as f32 / (TW / 2) as f32 - 1.0f32,
                            y as f32 / (TH / 2) as f32 - 1.0f32,
                        ) * 4.0,
                        2.0f32,
                    ) as i32)
                    .clamp(0, 50);
                let r = (!x & !y) & 255;
                let g = (x & !y) & 255;
                let b = (!x & y) & 255;
                bitmap.extend(
                    [
                        (l.max(r - d)).clamp(0, 255) as u8,
                        (l.max(g - d)).clamp(0, 255) as u8,
                        (l.max(b - d)).clamp(0, 255) as u8,
                        0u8,
                    ]
                    .iter(),
                );
            }
        }
        bitmap
    }
}

pub mod player_controller;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum AppState {
    // DebugMenu,
    InGame,
    Editor,
    // Paused,
}

pub fn exit_on_esc_system(
    keyboard_input: Res<Input<KeyCode>>,
    mut app_exit_events: EventWriter<AppExit>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        app_exit_events.send_default();
    }
}

mod components {
    use bevy::prelude::*;
    #[derive(Component)]
    pub struct DeferredMesh {
        pub mesh: Handle<Mesh>,
        pub material: Handle<StandardMaterial>,
        pub transform: Transform,
        pub id: String,
    }

    #[derive(Component)]
    pub struct IngameCamera;
}

mod systems {
    use std::{
        fs::File,
        io::{Read, Write},
        path::Path,
    };

    use bevy::{
        core_pipeline::fxaa::Fxaa,
        prelude::*,
        render::{camera::RenderTarget, view::RenderLayers},
    };
    use bevy_atmosphere::prelude::AtmosphereCamera;
    use bevy_rapier3d::prelude::*;
    use parry3d::shape::{ConvexPolyhedron, SharedShape};

    use crate::{
        components,
        editor::{self, resources},
        player_controller::{PlayerCamera, PlayerControllerBundle},
        render_layers, AppState,
    };

    pub fn update_deferred_mesh_system(
        mut commands: Commands,
        query: Query<(Entity, &components::DeferredMesh)>,
        meshes: Res<Assets<Mesh>>,
    ) {
        for (entity, deferred_mesh) in &query {
            if let Some(mesh) = meshes.get(&deferred_mesh.mesh) {
                let cache_dir = Path::new("cache");

                let collider = if let Ok(mut f) = File::open(cache_dir.join(&deferred_mesh.id)) {
                    // read pre-calculated collider
                    let mut buf = Vec::new();
                    f.read_to_end(&mut buf).unwrap();
                    let x: Vec<(Vec3, Quat, ConvexPolyhedron)> =
                        flexbuffers::from_slice(&buf[..]).unwrap();

                    Collider::compound(
                        x.into_iter()
                            .map(|(pos, rot, cp)| (pos, rot, SharedShape::new(cp).into()))
                            .collect(),
                    )
                } else {
                    // copmpute and store convex decomposition
                    let collider = Collider::from_bevy_mesh(
                        mesh,
                        &ComputedColliderShape::ConvexDecomposition(VHACDParameters::default()),
                    )
                    .unwrap();

                    if let Some(compound) = collider.as_compound() {
                        let x = compound
                            .shapes()
                            .filter_map(|(pos, rot, shape)| {
                                if let ColliderView::ConvexPolyhedron(ch) = shape {
                                    Some((pos, rot, ch))
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>();

                        let y = x
                            .iter()
                            .map(|(pos, rot, ch)| (pos, rot, ch.raw))
                            .collect::<Vec<_>>();

                        std::fs::create_dir_all(cache_dir).expect("could not create cache dir");
                        let mut f = File::create(cache_dir.join(&deferred_mesh.id)).unwrap();
                        let buf = flexbuffers::to_vec(y).unwrap();
                        f.write_all(&buf[..]).unwrap();
                    }
                    collider
                };

                commands
                    .entity(entity)
                    .remove::<components::DeferredMesh>()
                    .insert(PbrBundle {
                        mesh: deferred_mesh.mesh.clone(),
                        material: deferred_mesh.material.clone(),
                        transform: deferred_mesh.transform,
                        ..Default::default()
                    })
                    .insert(RigidBody::Dynamic)
                    .insert(collider)
                    .insert(Restitution {
                        coefficient: 0.2,
                        ..default()
                    })
                    .insert(ColliderScale::Absolute(Vec3::ONE))
                    .insert(ColliderMassProperties::Density(0.1));
            }
        }
    }

    pub fn toggle_debug_menu_system(
        key_codes: Res<Input<KeyCode>>,
        mut app_state: ResMut<State<AppState>>,
    ) {
        if key_codes.just_pressed(KeyCode::F3) {
            match app_state.current() {
                AppState::Editor => app_state.set(AppState::InGame).unwrap(),
                AppState::InGame => app_state.set(AppState::Editor).unwrap(),
            }
        }
    }

    pub fn setup_player_system(mut commands: Commands) {
        commands
            .spawn(SpatialBundle::from_transform(Transform::from_xyz(
                5.0, 2.01, 5.0,
            )))
            .insert(PlayerControllerBundle::default())
            .insert(Name::new("player"));
    }

    pub fn setup_debug_render_system(mut debug_render_context: ResMut<DebugRenderContext>) {
        // FIXME: for some reason this is enabled on startup, even though we insert initialize it explicity. Probably an ordering problem somewhere
        debug_render_context.enabled = false;
    }

    pub fn enter_editor_system(mut commands: Commands, wm_state: Res<resources::WmState>) {
        commands
            .spawn(Camera3dBundle {
                camera: Camera {
                    target: RenderTarget::Image(wm_state.slot_main3d.offscreen_image.clone()),
                    ..default()
                },
                ..default()
            })
            // .insert(Transform::from_xyz(5.0, 1.01, 10.0).looking_at(Vec3::new(0.0, 2.0, 0.0), Vec3::Y));
            // .insert(RenderPlayer(0))
            .insert(PlayerCamera)
            .insert(AtmosphereCamera::default())
            .insert(Fxaa::default())
            .insert(RenderLayers::layer(render_layers::MAIN_3D))
            .insert(editor::components::Main3dCamera);
    }
    pub fn leave_editor_system(
        mut commands: Commands,
        query: Query<Entity, With<editor::components::Main3dCamera>>,
    ) {
        for entity in &query {
            commands.entity(entity).despawn();
        }
    }

    pub fn enter_ingame_system(mut commands: Commands) {
        commands
            .spawn(Camera3dBundle { ..default() })
            .insert(PlayerCamera)
            .insert(AtmosphereCamera::default())
            .insert(Fxaa::default())
            .insert(RenderLayers::layer(render_layers::MAIN_3D))
            .insert(components::IngameCamera);
    }
    pub fn leave_ingame_system(
        mut commands: Commands,
        query: Query<Entity, With<components::IngameCamera>>,
    ) {
        for entity in &query {
            commands.entity(entity).despawn();
        }
    }
}

pub fn spawn_gltf2(
    commands: &mut Commands,
    asset_server: &AssetServer,
    filename: &str,
    position: Vec3,
    id: &str,
) {
    let bevy_path = format!("models/{}", filename);

    let mesh = asset_server.load(format!("{}#Mesh0/Primitive0", bevy_path));
    let material = asset_server.load(format!("{}#Material0", bevy_path));

    commands
        .spawn(components::DeferredMesh {
            mesh,
            material,
            transform: Transform::from_translation(position),
            id: id.to_string(),
        })
        .insert(Name::new("gltf"));
}

pub struct GameplayPlugin;
impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1.0 / 5.0f32,
        });
        app.add_startup_system(systems::setup_player_system);
        app.add_startup_system(systems::setup_debug_render_system);
        app.add_system(systems::update_deferred_mesh_system);
        app.add_state(AppState::InGame);
        app.add_system(systems::toggle_debug_menu_system);
        app.add_asset_loader(norm::NormalMappedImageTextureLoader);
        #[cfg(feature = "inspector")]
        {
            app.insert_resource(WorldInspectorParams {
                enabled: true,
                ..default()
            });
            app.add_plugin(bevy_inspector_egui::WorldInspectorPlugin::default());
            // app.add_plugin(bevy_inspector_egui_rapier::InspectableRapierPlugin);
        }
        app.add_system_set(
            SystemSet::on_enter(AppState::Editor).with_system(systems::enter_editor_system),
        );
        app.add_system_set(
            SystemSet::on_exit(AppState::Editor).with_system(systems::leave_editor_system),
        );

        app.add_system_set(
            SystemSet::on_enter(AppState::InGame).with_system(systems::enter_ingame_system),
        );
        app.add_system_set(
            SystemSet::on_exit(AppState::InGame).with_system(systems::leave_ingame_system),
        );

        // FIXME: those do not really belong here (related to external plugins)
        // Add material types to be converted
        app.add_system(bevy_mod_mipmap_generator::generate_mipmaps::<StandardMaterial>);
        app.insert_resource(DebugRenderContext {
            enabled: false,
            always_on_top: false,
            ..default()
        });
        // app.insert_resource(Msaa { samples: 1 });
        app.insert_resource(ClearColor(Color::ALICE_BLUE));
    }
}

pub struct GamePluginGroup;
impl PluginGroup for GamePluginGroup {
    fn build(self) -> bevy::app::PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(player_controller::PlayerControllerPlugin)
            .add(GameplayPlugin)
    }
}

pub struct EditorPluginGroup;
impl PluginGroup for EditorPluginGroup {
    fn build(self) -> bevy::app::PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(editor::EditorPlugin)
            .add(debug_gui::DebugGuiPlugin)
    }
}

pub struct ExternalPluginGroup;
impl PluginGroup for ExternalPluginGroup {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(RapierPhysicsPlugin::<NoUserData>::default())
            // .add(RapierDebugRenderPlugin::default())
            .add(bevy_prototype_debug_lines::DebugLinesPlugin::default())
            .add(FrameTimeDiagnosticsPlugin)
            .add(sky::SkyPlugin)
            .add(bevy_mod_mipmap_generator::MipmapGeneratorPlugin)
            .add(bevy_mod_outline::OutlinePlugin)
            .add(bevy_infinite_grid::InfiniteGridPlugin)
    }
}
