use std::collections::HashMap;

use bevy::{
    prelude::*,
    render::{
        mesh,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
};
use bevy_rapier3d::{na::OPoint, prelude::*};
use gltf::Semantic;
use physics::test_texture;

fn main() {
    App::new()
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1.0 / 5.0f32,
        })
        .insert_resource(Msaa::default())
        .add_plugins(DefaultPlugins)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugin(RapierRenderPlugin)
        .add_startup_system(setup)
        .add_system(animate_light_direction)
        .add_system(rotation_system)
        // .add_system(mesh_loaded)
        .run();
}

fn spawn_sphere(
    mut commands: &mut Commands,
    radius: f32,
    position: Vec3,
    material: Handle<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
) {
    let rigid_body = RigidBodyBundle {
        forces: RigidBodyForces {
            gravity_scale: 1.0,
            ..Default::default()
        }
        .into(),
        position: position.into(),
        ..Default::default()
    };
    let collider = ColliderBundle {
        shape: ColliderShape::ball(radius).into(),
        material: ColliderMaterial {
            restitution: 0.2,
            ..Default::default()
        }
        .into(),
        ..Default::default()
    };
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
            // transform: Transform::from_translation(Vec3::new(0.0, 1.0, 0.0)),
            ..Default::default()
        })
        .insert(Rotation { vel: 1.0 })
        .insert_bundle(rigid_body)
        .insert_bundle(collider)
        .insert(RigidBodyPositionSync::Discrete);
}
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
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

    let material = materials.add(StandardMaterial {
        base_color_texture: Some(uv_test),
        metallic: 0.9,
        perceptual_roughness: 0.1,
        ..Default::default()
    });

    // .insert(ColliderDebugRender::with_id(1));

    spawn_sphere(
        &mut commands,
        0.5,
        Vec3::new(-2.2, 15.0, 0.0),
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

    let collider = ColliderBundle {
        shape: ColliderShape::cuboid(100.0, 0.1, 100.0).into(),

        ..Default::default()
    };
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(
                mesh::shape::Box {
                    min_x: -50.0,
                    max_x: 50.0,
                    min_y: 0.0,
                    max_y: 0.1,
                    min_z: -50.0,
                    max_z: 50.0,
                }
                .into(),
            ),
            material,
            ..Default::default()
        })
        .insert_bundle(collider);

    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(5.7, 1.7, 5.0).looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::Y),
        ..Default::default()
    });
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

    // commands.spawn_bundle(PointLightBundle {
    //     point_light: PointLight {
    //         shadows_enabled: true,
    //         ..Default::default()
    //     },
    //     transform: Transform::from_translation(Vec3::new(0.0, 2.0, -3.0)),
    //     ..Default::default()
    // });

    let (document, buffers, images) = gltf::import("assets/models/anvil_gltf/anvil.gltf").unwrap();
    let mut anvil_collider = None;
    for mesh in document.meshes() {
        println!("Mesh #{}", mesh.index());
        for primitive in mesh.primitives() {
            println!("- Primitive #{} {:?}", primitive.index(), primitive.mode());
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()].0));
            let pos = reader
                .read_positions()
                .unwrap()
                .map(|p| nalgebra::Point3::new(p[0], p[1], p[2]))
                .collect::<Vec<_>>();
            let indices = reader
                .read_indices()
                .unwrap()
                .into_u32()
                .collect::<Vec<_>>();

            let indices = indices
                .chunks(3)
                .map(|c| [c[0], c[1], c[2]])
                .collect::<Vec<_>>();

            let collider = ColliderShape::convex_decomposition(&pos[..], &indices[..]);
            println!("collider: {:?}", collider.as_compound().unwrap().aabbs());
            anvil_collider = Some(collider);
        }
    }

    let rigid_body = RigidBodyBundle {
        forces: RigidBodyForces {
            gravity_scale: 1.0,
            ..Default::default()
        }
        .into(),
        position: Vec3::new(-0.3, 5.0, -0.5).into(),
        ..Default::default()
    };
    let collider = ColliderBundle {
        shape: anvil_collider.unwrap().into(),
        material: ColliderMaterial {
            restitution: 0.2,
            ..Default::default()
        }
        .into(),
        ..Default::default()
    };

    let anvil_mesh = asset_server.load("models/anvil_gltf/anvil.gltf#Mesh0/Primitive0");
    let anvil_material = asset_server.load("models/anvil_gltf/anvil.gltf#Material0");

    commands
        .spawn_bundle(PbrBundle {
            mesh: anvil_mesh,
            material: anvil_material,
            // transform: Transform::from_translation(Vec3::new(0.0, 1.0, 0.0)),
            ..Default::default()
        })
        .insert(Rotation { vel: 1.0 })
        .insert_bundle(rigid_body)
        .insert_bundle(collider)
        // .insert(ColliderDebugRender::default())
        .insert(RigidBodyPositionSync::Discrete);

    // commands.spawn_scene(asset_server.load("models/anvil_gltf/anvil.gltf#Scene0"));
    // let anvil_mesh: Handle<Mesh> =
    //     asset_server.load(&format!("models/anvil_gltf/anvil.gltf#Mesh0/Primitive0"));
    // anvil_mesh.
    // anvil_mesh.
}

fn animate_light_direction(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<DirectionalLight>>,
) {
    for mut transform in query.iter_mut() {
        transform.rotation = Quat::from_euler(
            EulerRot::ZYX,
            0.0,
            time.seconds_since_startup() as f32 * std::f32::consts::TAU / 10.0,
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
