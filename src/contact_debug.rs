use crate::trace;

use bevy::{math::Vec3, prelude::*};

#[derive(Component)]
pub struct ContactDebugMesh {
    elapsed: Timer,
}

#[derive(Default, Resource)]
pub struct ContactDebug {
    pub add: Vec<(trace::TraceContact, Vec3)>,
    add_pointer: Vec<(Vec3, Vec3)>,
    plane_mesh: Option<Handle<Mesh>>,
}

pub fn contact_debug(
    time: Res<Time>,
    mut commands: Commands,
    mut contact_debug: ResMut<ContactDebug>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut reaper_query: Query<(Entity, &mut ContactDebugMesh)>,
) {
    let mut cv = Vec::new();
    std::mem::swap(&mut contact_debug.add, &mut cv);
    for (contact, shape_origin) in cv.drain(..) {
        let mesh = contact_debug
            .plane_mesh
            // .get_or_insert_with(|| meshes.add(mesh::shape::Quad::new(Vec2::new(0.1, 0.1)).into()))
            .get_or_insert_with(|| {
                meshes.add(
                    Capsule3d::new(0.01, 0.1)
                        .mesh()
                        .latitudes(2)
                        .longitudes(3)
                        .rings(2)
                        .build(),
                )
            })
            .clone();

        let rotation = Quat::from_rotation_arc(Vec3::Y, contact.collider_normal);
        commands
            .spawn(PbrBundle {
                mesh,
                transform: Transform::from_translation(shape_origin + contact.shape_point)
                    .with_rotation(rotation),
                ..Default::default()
            })
            .insert(ContactDebugMesh {
                elapsed: Timer::from_seconds(5.0, TimerMode::Repeating),
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
                    Capsule3d::new(0.01, 0.1)
                        .mesh()
                        .latitudes(2)
                        .longitudes(3)
                        .rings(2)
                        .build(),
                )
            })
            .clone();

        let rotation = Quat::from_rotation_arc(Vec3::Y, vec);
        commands
            .spawn(PbrBundle {
                mesh,
                transform: Transform::from_translation(pos).with_rotation(rotation),
                ..Default::default()
            })
            .insert(ContactDebugMesh {
                elapsed: Timer::from_seconds(5.0, TimerMode::Once),
            });
    }

    for (entity, mut dbg_mesh) in reaper_query.iter_mut() {
        dbg_mesh.elapsed.tick(time.delta());
        if dbg_mesh.elapsed.just_finished() {
            commands.entity(entity).despawn();
        }
    }
}
