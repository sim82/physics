use bevy::{
    app::{AppExit, PluginGroupBuilder},
    color::palettes::tailwind,
    diagnostic::FrameTimeDiagnosticsPlugin,
    pbr::wireframe::WireframePlugin,
    prelude::*,
    window::PresentMode,
};
#[cfg(feature = "inspector")]
// use bevy_inspector_egui::WorldInspectorParams;
use bevy_rapier3d::prelude::*;
use shared::AppState;

pub mod appearance;
pub mod contact_debug;
pub mod slidemove;
pub mod trace;

#[cfg(feature = "atmosphere")]
pub mod sky;

pub mod norm {
    use bevy::asset::io::Reader;
    // srgb workaround from https://github.com/bevyengine/bevy/issues/6371
    use bevy::asset::{AssetLoader, AsyncReadExt, LoadContext};
    use bevy::prelude::*;
    use bevy::render::texture::{CompressedImageFormats, Image, ImageType};
    use bevy::utils::BoxedFuture;

    #[derive(Default)]
    pub struct NormalMappedImageTextureLoader;

    impl AssetLoader for NormalMappedImageTextureLoader {
        fn load<'a>(
            &'a self,
            reader: &'a mut Reader,
            _settings: &'a Self::Settings,
            _load_context: &'a mut LoadContext,
        ) -> BoxedFuture<'a, Result<Self::Asset, anyhow::Error>> {
            Box::pin(async move {
                let mut bytes = Vec::new();
                reader.read_to_end(&mut bytes).await?;
                let dyn_img = Image::from_buffer(
                    bytes.as_slice(),
                    ImageType::Extension("png"),
                    CompressedImageFormats::all(),
                    false,
                    default(),
                    default(),
                )
                .unwrap();

                // load_context.set_default_asset(LoadedAsset::new(dyn_img));
                // Ok(())
                Ok(dyn_img)
            })
        }

        fn extensions(&self) -> &[&str] {
            &["norm"]
        }

        type Asset = Image;

        type Settings = ();

        type Error = anyhow::Error;
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

pub fn exit_on_esc_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut app_exit_events: EventWriter<AppExit>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        app_exit_events.send_default();
    }
}

mod resources {
    use bevy::prelude::Resource;

    #[derive(Resource, Copy, Clone, Default, Debug)]
    pub enum AaState {
        #[default]
        Msaa4,
        Fxaa,
        Disabled,
    }
    impl AaState {
        pub fn next(self) -> Self {
            match self {
                AaState::Msaa4 => AaState::Fxaa,
                AaState::Fxaa => AaState::Disabled,
                AaState::Disabled => AaState::Msaa4,
            }
        }
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
        window::{CursorGrabMode, PrimaryWindow},
    };
    #[cfg(feature = "atmosphere")]
    use bevy_atmosphere::plugin::AtmosphereCamera;
    use bevy_rapier3d::{
        prelude::*,
        rapier::prelude::{ConvexPolyhedron, SharedShape},
    };
    // use parry3d::shape::{ConvexPolyhedron, SharedShape};

    use crate::{
        components,
        player_controller::{PlayerCamera, PlayerControllerBundle},
        resources,
    };
    use shared::{render_layers, AppState};

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
        key_codes: Res<ButtonInput<KeyCode>>,
        app_state: Res<State<AppState>>,
        mut next_state: ResMut<NextState<AppState>>,
    ) {
        if key_codes.just_pressed(KeyCode::F3) {
            match app_state.get() {
                AppState::Editor => next_state.set(AppState::InGame),
                AppState::InGame => next_state.set(AppState::Editor),
            }
        }
    }

    pub fn setup_player_system(
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        let player_mesh = meshes.add(Cuboid::new(0.6, 1.8, 0.6).mesh());
        let asset: StandardMaterial = Color::rgba(0.8, 0.8, 0.4, 0.4).into();
        let player_material = materials.add(asset);

        commands
            .spawn(SpatialBundle::from_transform(Transform::from_xyz(
                5.0, 2.01, 5.0,
            )))
            .insert(PlayerControllerBundle::default())
            .insert(Name::new("player"))
            .with_children(|commands| {
                commands.spawn((
                    PbrBundle {
                        mesh: player_mesh,
                        material: player_material,
                        ..default()
                    },
                    render_layers::ortho_views(),
                ));
            });
    }

    pub fn setup_debug_render_system(mut debug_render_context: ResMut<DebugRenderContext>) {
        // FIXME: for some reason this is enabled on startup, even though we insert initialize it explicity. Probably an ordering problem somewhere
        debug_render_context.enabled = false;
    }

    pub fn enter_editor_system(mut commands: Commands, wm_state: Res<editor::resources::WmState>) {
        let mut entity_commands = commands.spawn(Camera3dBundle {
            camera: Camera {
                target: RenderTarget::Image(wm_state.slot_main3d.offscreen_image.clone()),
                ..default()
            },
            ..default()
        });
        // .insert(Transform::from_xyz(5.0, 1.01, 10.0).looking_at(Vec3::new(0.0, 2.0, 0.0), Vec3::Y));
        // .insert(RenderPlayer(0))
        entity_commands
            .insert(PlayerCamera)
            .insert(Fxaa::default())
            .insert(RenderLayers::layer(render_layers::MAIN_3D))
            .insert(editor::components::Main3dCamera);

        #[cfg(feature = "atmosphere")]
        entity_commands.insert(AtmosphereCamera::default());
    }
    pub fn leave_editor_system(
        mut commands: Commands,
        query: Query<Entity, With<editor::components::Main3dCamera>>,
    ) {
        for entity in &query {
            commands.entity(entity).despawn();
        }
    }

    pub fn enter_ingame_system(
        mut commands: Commands,
        mut primary_query: Query<&mut Window, With<PrimaryWindow>>,
    ) {
        let mut entitiy_commands = commands.spawn(Camera3dBundle { ..default() });
        entitiy_commands
            .insert(PlayerCamera)
            .insert(Fxaa {
                enabled: false,
                ..default()
            })
            .insert(RenderLayers::layer(render_layers::MAIN_3D))
            .insert(components::IngameCamera);

        #[cfg(feature = "atmosphere")]
        entitiy_commands.insert(AtmosphereCamera::default());

        if let Ok(mut window) = primary_query.get_single_mut() {
            window.cursor.grab_mode = CursorGrabMode::Locked;
        };
    }
    pub fn leave_ingame_system(
        mut commands: Commands,
        mut primary_query: Query<&mut Window, With<PrimaryWindow>>,
        query: Query<Entity, With<components::IngameCamera>>,
    ) {
        for entity in &query {
            commands.entity(entity).despawn();
        }
        if let Ok(mut window) = primary_query.get_single_mut() {
            window.cursor.grab_mode = CursorGrabMode::None;
        }
    }
    pub fn toggle_anti_aliasing(
        mut state: ResMut<resources::AaState>,
        key_codes: Res<ButtonInput<KeyCode>>,
        mut msaa: ResMut<Msaa>,
        mut query: Query<&mut Fxaa>,
    ) {
        if key_codes.just_pressed(KeyCode::F11) {
            *state = state.next();
            info!("antialias state: {:?}", state);
            for mut fxaa in &mut query {
                match *state {
                    resources::AaState::Msaa4 => {
                        *msaa = Msaa::Sample4;
                        fxaa.enabled = false
                    }
                    resources::AaState::Fxaa => {
                        *msaa = Msaa::Off;
                        fxaa.enabled = true;
                    }
                    resources::AaState::Disabled => {
                        *msaa = Msaa::Off;
                        fxaa.enabled = false;
                    }
                }
            }
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
            brightness: 100.0,
        });
        app.insert_resource(DebugRenderContext {
            enabled: false,
            // always_on_top: false,
            ..default()
        });
        // app.insert_resource(PresentMode::Mailbox);
        app.insert_resource(ClearColor(tailwind::BLUE_200.into()));
        app.init_resource::<resources::AaState>();
        app.add_systems(Startup, systems::setup_player_system);
        app.add_systems(Startup, systems::setup_debug_render_system);
        app.add_systems(Update, systems::update_deferred_mesh_system);
        app.insert_state(AppState::default());
        app.add_systems(Update, systems::toggle_debug_menu_system);
        app.register_asset_loader(norm::NormalMappedImageTextureLoader);
        #[cfg(feature = "inspector")]
        {
            app.add_plugins(bevy_inspector_egui::DefaultInspectorConfigPlugin);
        }
        app.add_plugins(bevy_egui::EguiPlugin);
        app.add_systems(OnEnter(AppState::Editor), systems::enter_editor_system);
        app.add_systems(OnExit(AppState::Editor), systems::leave_editor_system);
        app.add_systems(OnEnter(AppState::InGame), systems::enter_ingame_system);
        app.add_systems(OnExit(AppState::InGame), systems::leave_ingame_system);

        // FIXME: those do not really belong here (related to external plugins)
        // Add material types to be converted
        app.add_systems(
            Update,
            bevy_mod_mipmap_generator::generate_mipmaps::<StandardMaterial>,
        );
        app.add_systems(Update, systems::toggle_anti_aliasing);
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

pub struct ExternalPluginGroup;
impl PluginGroup for ExternalPluginGroup {
    fn build(self) -> PluginGroupBuilder {
        let builder = PluginGroupBuilder::start::<Self>()
            .add(RapierPhysicsPlugin::<NoUserData>::default())
            .add(WireframePlugin)
            // .add(RapierDebugRenderPlugin::default())
            .add(FrameTimeDiagnosticsPlugin)
            .add(bevy_mod_mipmap_generator::MipmapGeneratorPlugin);

        #[cfg(feature = "atmosphere")]
        let builder = builder.add(sky::SkyPlugin);

        #[allow(clippy::let_and_return)]
        builder
    }
}
