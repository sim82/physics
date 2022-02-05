use crate::{debug_lines, trace};

use bevy::{math::Vec3, prelude::*, render::mesh};

#[derive(Component)]
pub struct ContactDebugMesh {
    elapsed: Timer,
}

#[derive(Default)]
pub struct ContactDebug {
    pub add: Vec<(trace::Contact, Vec3)>,
    add_pointer: Vec<(Vec3, Vec3)>,
    plane_mesh: Option<Handle<Mesh>>,
}

pub fn contact_debug(
    time: Res<Time>,
    mut commands: Commands,
    mut contact_debug: ResMut<ContactDebug>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut reaper_query: Query<(Entity, &mut ContactDebugMesh)>,
    mut _debug_lines: ResMut<debug_lines::DebugLines>,
) {
    let mut cv = Vec::new();
    std::mem::swap(&mut contact_debug.add, &mut cv);
    for (contact, shape_origin) in cv.drain(..) {
        let mesh = contact_debug
            .plane_mesh
            // .get_or_insert_with(|| meshes.add(mesh::shape::Quad::new(Vec2::new(0.1, 0.1)).into()))
            .get_or_insert_with(|| {
                meshes.add(
                    mesh::shape::Capsule {
                        radius: 0.01,
                        depth: 0.1,
                        latitudes: 2,
                        longitudes: 3,
                        rings: 2,
                        ..Default::default()
                    }
                    .into(),
                )
            })
            .clone();

        let rotation = Quat::from_rotation_arc(Vec3::Y, contact.collider_normal);
        commands
            .spawn_bundle(PbrBundle {
                mesh,
                transform: Transform::from_translation(shape_origin + contact.shape_point)
                    .with_rotation(rotation),
                ..Default::default()
            })
            .insert(ContactDebugMesh {
                elapsed: Timer::from_seconds(5.0, false),
            });
    }

    let mut cv = Vec::new();
    std::mem::swap(&mut contact_debug.add_pointer, &mut cv);
    for (pos, vec) in cv.drain(..) {
        let mesh = contact_debug
            .plane_mesh
            // .get_or_insert_with(|| meshes.add(mesh::shape::Quad::new(Vec2::new(0.1, 0.1)).into()))
            .get_or_insert_with(|| {
                meshes.add(
                    mesh::shape::Capsule {
                        radius: 0.01,
                        depth: 0.1,
                        latitudes: 2,
                        longitudes: 3,
                        rings: 2,
                        ..Default::default()
                    }
                    .into(),
                )
            })
            .clone();

        let rotation = Quat::from_rotation_arc(Vec3::Y, vec);
        commands
            .spawn_bundle(PbrBundle {
                mesh,
                transform: Transform::from_translation(pos).with_rotation(rotation),
                ..Default::default()
            })
            .insert(ContactDebugMesh {
                elapsed: Timer::from_seconds(5.0, false),
            });
    }

    for (entity, mut dbg_mesh) in reaper_query.iter_mut() {
        dbg_mesh.elapsed.tick(time.delta());
        if dbg_mesh.elapsed.just_finished() {
            commands.entity(entity).despawn();
        }
    }
}
