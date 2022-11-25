use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};

use bevy::{
    diagnostic::FrameTimeDiagnosticsPlugin,
    pbr::wireframe::WireframePlugin,
    // input::system::exit_on_esc_system,
    prelude::*,
    render::{
        mesh::{self},
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
};
use bevy_atmosphere::prelude::*;
// use bevy_editor_pls::prelude::*;
use bevy_prototype_debug_lines::DebugLines;
use bevy_rapier3d::prelude::*;
use parry3d::shape::{ConvexPolyhedron, SharedShape};
use physics::{
    editor::{self, util::spawn_box},
    norm,
    player_controller::{self, PlayerCamera, PlayerControllerBundle},
    sky, test_texture, AppState, TestResources,
};

#[cfg(feature = "inspector")]
use bevy_inspector_egui::WorldInspectorParams;

// use bevy_fps_controller::controller::*;

fn main() {
    let mut app = App::new();
    // .insert_resource(WindowDescriptor {
    //     mode: bevy::window::WindowMode::Fullscreen,
    //     width: 1280.0,
    //     height: 720.0,
    //     ..Default::default()
    // })
    app.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 1.0 / 5.0f32,
    })
    // .insert_resource(Msaa::default())
    .add_plugins(DefaultPlugins.set(ImagePlugin {
        default_sampler: wgpu::SamplerDescriptor {
            // mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            ..Default::default()
        },
    }))
    .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
    .add_plugin(RapierDebugRenderPlugin::default())
    .add_plugin(bevy_prototype_debug_lines::DebugLinesPlugin::default())
    // .add_plugin(EditorPlugin)
    .add_startup_system(setup)
    .add_system(animate_light_direction)
    .add_system(rotation_system)
    // .add_plugin(physics::CharacterStateInputPlugin::default())
    .add_plugin(player_controller::PlayerControllerPlugin)
    // .add_system(exit_on_esc_system)
    .add_system(bevy::window::close_on_esc)
    .add_plugin(FrameTimeDiagnosticsPlugin)
    .add_system(debug_line_test)
    .add_system(update_deferred_mesh_system)
    // .add_plugin(FpsControllerPlugin)
    // .add_plugin(AtmospherePlugin)
    // .add_plugin(WireframePlugin)
    .add_plugin(sky::SkyPlugin)
    // .insert_resource(Atmosphere {
    //     ray_origin: Vec3::new(0.0, 7000e3, 0.0),
    //     planet_radius: 7000e3,
    //     atmosphere_radius: 7100e3,
    //     ..default()
    // })
    .init_resource::<TestResources>();
    // .add_system(mesh_loaded)
    app.add_state(AppState::DebugMenu);
    app.add_system(toggle_debug_menu_system);

    app.add_plugin(editor::EditorPlugin);
    app.add_asset_loader(norm::NormalMappedImageTextureLoader);

    #[cfg(feature = "inspector")]
    {
        app.insert_resource(WorldInspectorParams {
            enabled: false,
            ..default()
        });
        app.add_plugin(bevy_inspector_egui::WorldInspectorPlugin::default());
        // app.add_plugin(bevy_inspector_egui_rapier::InspectableRapierPlugin);
        app.add_system_set(
            SystemSet::on_enter(AppState::DebugMenu).with_system(open_world_inspector),
        );
        app.add_system_set(
            SystemSet::on_exit(AppState::DebugMenu).with_system(close_world_inspector),
        );
    }

    app.run();

    info!("after app.run");
}

#[cfg(feature = "inspector")]
fn open_world_inspector(mut inspector_params: ResMut<WorldInspectorParams>) {
    inspector_params.enabled = true;
}

#[cfg(feature = "inspector")]
fn close_world_inspector(mut inspector_params: ResMut<WorldInspectorParams>) {
    inspector_params.enabled = false;
}

fn toggle_debug_menu_system(
    key_codes: Res<Input<KeyCode>>,
    mut app_state: ResMut<State<AppState>>,
) {
    if key_codes.just_pressed(KeyCode::F3) {
        match app_state.current() {
            AppState::DebugMenu => app_state.set(AppState::InGame).unwrap(),
            AppState::InGame => app_state.set(AppState::DebugMenu).unwrap(),
        }
    }
}

fn spawn_sphere(
    commands: &mut Commands,
    radius: f32,
    position: Vec3,
    material: Handle<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
) {
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(
                mesh::shape::Icosphere {
                    radius,
                    subdivisions: 6,
                }
                .into(),
            ),
            material,
            transform: Transform::from_translation(position),
            ..Default::default()
        })
        .insert(Rotation { vel: 1.0 })
        .insert(RigidBody::Dynamic)
        .insert(Collider::ball(radius))
        .insert(Restitution {
            coefficient: 0.2,
            ..default()
        })
        .insert(ColliderScale::Absolute(Vec3::ONE));
    // .insert(Rigid);
}

#[derive(Component)]
struct DeferredMesh {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    transform: Transform,
    id: String,
}

fn spawn_gltf2(
    commands: &mut Commands,
    asset_server: &AssetServer,
    filename: &str,
    position: Vec3,
    id: &str,
) {
    let bevy_path = format!("models/{}", filename);

    let mesh = asset_server.load(&format!("{}#Mesh0/Primitive0", bevy_path));
    let material = asset_server.load(&format!("{}#Material0", bevy_path));

    commands
        .spawn(DeferredMesh {
            mesh,
            material,
            transform: Transform::from_translation(position),
            id: id.to_string(),
        })
        .insert(Name::new("gltf"));
}

fn update_deferred_mesh_system(
    mut commands: Commands,
    query: Query<(Entity, &DeferredMesh)>,
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
                    let buf = flexbuffers::to_vec(&y).unwrap();
                    f.write_all(&buf[..]).unwrap();
                }
                collider
            };

            commands
                .entity(entity)
                .remove::<DeferredMesh>()
                .insert_bundle(PbrBundle {
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

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut test_resources: ResMut<TestResources>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut _debug_lines: ResMut<bevy_prototype_debug_lines::DebugLines>,
) {
    let uv_test = images.add(Image::new(
        Extent3d {
            width: test_texture::TW as u32,
            height: test_texture::TH as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        test_texture::create(),
        TextureFormat::Rgba8Unorm,
    ));
    test_resources.uv_image = uv_test.clone();

    let material = materials.add(StandardMaterial {
        base_color_texture: Some(uv_test),
        metallic: 0.9,
        perceptual_roughness: 0.1,
        ..Default::default()
    });

    test_resources.uv_material = material.clone();

    const SPAWN_STUFF: bool = false;
    // .insert(ColliderDebugRender::with_id(1));
    if SPAWN_STUFF {
        spawn_sphere(
            &mut commands,
            0.5,
            Vec3::new(-0.1, 5.0, 0.0),
            material.clone(),
            &mut meshes,
        );
        spawn_sphere(
            &mut commands,
            0.7,
            Vec3::new(1.5, 25.0, 0.0),
            material.clone(),
            &mut meshes,
        );

        // groundplane
        const GROUND_PLANE_HALFSIZE: f32 = 25.0;
        spawn_box(
            &mut commands,
            material.clone(),
            &mut meshes,
            Vec3::new(-GROUND_PLANE_HALFSIZE, -0.1, -GROUND_PLANE_HALFSIZE),
            Vec3::new(GROUND_PLANE_HALFSIZE, 0.0, GROUND_PLANE_HALFSIZE),
        );

        // x-fence
        spawn_box(
            &mut commands,
            material.clone(),
            &mut meshes,
            Vec3::new(-GROUND_PLANE_HALFSIZE, 0.0, -GROUND_PLANE_HALFSIZE),
            Vec3::new(GROUND_PLANE_HALFSIZE, 0.5, -GROUND_PLANE_HALFSIZE + 0.1),
        );

        spawn_box(
            &mut commands,
            material.clone(),
            &mut meshes,
            Vec3::new(-GROUND_PLANE_HALFSIZE, 0.0, GROUND_PLANE_HALFSIZE - 0.1),
            Vec3::new(GROUND_PLANE_HALFSIZE, 0.5, GROUND_PLANE_HALFSIZE),
        );

        // z-fence
        spawn_box(
            &mut commands,
            material.clone(),
            &mut meshes,
            Vec3::new(-GROUND_PLANE_HALFSIZE, 0.0, -GROUND_PLANE_HALFSIZE),
            Vec3::new(-GROUND_PLANE_HALFSIZE + -0.1, 0.5, GROUND_PLANE_HALFSIZE),
        );

        spawn_box(
            &mut commands,
            material.clone(),
            &mut meshes,
            Vec3::new(GROUND_PLANE_HALFSIZE - 0.1, 0.0, -GROUND_PLANE_HALFSIZE),
            Vec3::new(GROUND_PLANE_HALFSIZE, 0.5, GROUND_PLANE_HALFSIZE),
        );

        if false {
            // build stairs
            {
                let mut x = 5.0;
                let mut y = 0.0;
                let mut z = 5.0;

                for _ in 0..10 {
                    spawn_box(
                        &mut commands,
                        material.clone(),
                        &mut meshes,
                        Vec3::new(-x, y, -z),
                        Vec3::new(x, y + 0.1, z),
                    );
                    x -= 0.4;
                    z -= 0.4;
                    y += 0.1;
                }
            }
        }

        spawn_box(
            &mut commands,
            material,
            &mut meshes,
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(5.0, 2.5, 0.3),
        );
    }
    commands
        .spawn_bundle(SpatialBundle::default())
        .insert_bundle(PlayerControllerBundle::default())
        .insert(Name::new("player"))
        .insert(
            Transform::from_xyz(10.0, 1.01, 10.0), //.looking_at(Vec3::new(0.0, 2.0, 0.0), Vec3::Y),
        );

    commands
        .spawn_bundle(Camera3dBundle::default())
        // .insert(Transform::from_xyz(5.0, 1.01, 10.0).looking_at(Vec3::new(0.0, 2.0, 0.0), Vec3::Y));
        // .insert(RenderPlayer(0))
        .insert(PlayerCamera)
        .insert(AtmosphereCamera(None));

    if SPAWN_STUFF {
        const HALF_SIZE: f32 = 5.0;
        commands.spawn_bundle(DirectionalLightBundle {
            directional_light: DirectionalLight {
                shadow_projection: OrthographicProjection {
                    left: -HALF_SIZE,
                    right: HALF_SIZE,
                    bottom: -HALF_SIZE,
                    top: HALF_SIZE,
                    near: -10.0 * HALF_SIZE,
                    far: 10.0 * HALF_SIZE,
                    ..Default::default()
                },
                shadows_enabled: true,
                ..Default::default()
            },
            ..Default::default()
        });

        spawn_gltf2(
            &mut commands,
            &asset_server,
            "donut_gltf/donut.gltf",
            Vec3::new(-0.1, 2.0, -1.0),
            "donut",
        );

        spawn_gltf2(
            &mut commands,
            &asset_server,
            "anvil_gltf/anvil.gltf",
            Vec3::new(-0.1, 7.0, -1.0),
            "anvil",
        );
    }
}

fn animate_light_direction(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<DirectionalLight>>,
) {
    for mut transform in query.iter_mut() {
        transform.rotation = Quat::from_euler(
            EulerRot::ZYX,
            0.0,
            time.elapsed_seconds() as f32 * std::f32::consts::TAU / 10.0,
            -std::f32::consts::FRAC_PI_4,
        );
    }
}

#[derive(Component)]
struct Rotation {
    vel: f32,
}

fn rotation_system(time: Res<Time>, mut query: Query<(&mut Transform, &Rotation)>) {
    for (mut transform, rotation) in query.iter_mut() {
        transform.rotation *= Quat::from_rotation_y(rotation.vel * 1.0e-1 * time.delta_seconds());
    }
}

fn debug_line_test(time: Res<Time>, mut lines: ResMut<DebugLines>) {
    let seconds = time.elapsed_seconds() as f32;
    let offset = Vec3::new(20.0, 0.0, 20.0);
    lines.line(
        Vec3::new(-1.0, 2.0 + f32::sin(seconds), -1.0) + offset,
        Vec3::new(1.0, 2.0 + f32::sin(seconds + std::f32::consts::PI), 1.0) + offset,
        5.0,
    );
}
