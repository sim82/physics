use std::{collections::VecDeque, time::Duration};

use bevy::{input::mouse::MouseMotion, math::Vec3, prelude::*, render::mesh};
// use bevy_rapier3d::physics::{
//     QueryPipelineColliderComponentsQuery, QueryPipelineColliderComponentsSet,
// };
use bevy_rapier3d::prelude::*;

pub mod test_texture {
    pub const TW: usize = 256;
    pub const TH: usize = 256;

    pub fn create() -> Vec<u8> {
        // let mut bitmap = [0u32; TW * TH];

        let mut bitmap = Vec::new();

        for y in 0..TH as i32 {
            for x in 0..TW as i32 {
                let l = (0x1FF
                    >> [x, y, TW as i32 - 1 - x, TH as i32 - 1 - y, 31]
                        .iter()
                        .min()
                        .unwrap()) as i32;

                // std::cmp::min
                let d = std::cmp::min(
                    50,
                    std::cmp::max(
                        0,
                        255 - 50
                            * f32::powf(
                                f32::hypot(
                                    x as f32 / (TW / 2) as f32 - 1.0f32,
                                    y as f32 / (TH / 2) as f32 - 1.0f32,
                                ) * 4.0,
                                2.0f32,
                            ) as i32,
                    ),
                );
                let r = (!x & !y) & 255;
                let g = (x & !y) & 255;
                let b = (!x & y) & 255;
                // let color = std::cmp::min(std::cmp::max(r - d, l), 255) * 65536
                //     + std::cmp::min(std::cmp::max(g - d, l), 255) * 256
                //     + std::cmp::min(std::cmp::max(b - d, l), 255);
                // bitmap[y as usize * TW + x as usize] = color as u32;

                bitmap.extend([r as u8, g as u8, b as u8, 0u8].iter());
            }
        }
        bitmap
    }
}

#[derive(Component)]
pub struct InputMapping {
    forward: KeyCode,
    backward: KeyCode,
    strafe_right: KeyCode,
    strafe_left: KeyCode,
    walk: KeyCode,
}

impl Default for InputMapping {
    fn default() -> Self {
        Self {
            forward: KeyCode::W,
            backward: KeyCode::S,
            strafe_right: KeyCode::D,
            strafe_left: KeyCode::A,
            walk: KeyCode::LShift,
        }
    }
}

impl InputMapping {
    pub fn is_forward(&self, input: &Input<KeyCode>) -> bool {
        input.pressed(self.forward)
    }

    pub fn is_backward(&self, input: &Input<KeyCode>) -> bool {
        input.pressed(self.backward)
    }

    pub fn is_strafe_right(&self, input: &Input<KeyCode>) -> bool {
        input.pressed(self.strafe_right)
    }

    pub fn is_strafe_left(&self, input: &Input<KeyCode>) -> bool {
        input.pressed(self.strafe_left)
    }

    pub fn is_walk(&self, input: &Input<KeyCode>) -> bool {
        input.pressed(self.walk)
    }
}

struct InputState {
    time: Time,
    serial: usize,
    forward: bool,
    backward: bool,
    strafe_right: bool,
    strafe_left: bool,
    walk: bool,
    delta_pitch: f32,
    delta_yaw: f32,
}

#[derive(Default)]
struct InputStateQueue {
    queue: VecDeque<InputState>,
    next_serial: usize,
}

impl InputStateQueue {
    pub fn push(&mut self, mut input_state: InputState) {
        input_state.serial = self.next_serial;
        self.next_serial += 1;
        self.queue.push_back(input_state);
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }
    pub fn iter(&self) -> impl Iterator<Item = &InputState> {
        self.queue.iter()
    }

    pub fn retire_up_to(&mut self, serial: usize) {
        while let Some(front) = self.queue.front() {
            if front.serial <= serial {
                self.queue.pop_front();
            }
            // println!("front: {}", front.serial);
        }
    }

    // pub fn drain_newer(&mut self) -> impl IntoIterator<Item = InputState> {
    //     let tmp = self.queue.drain(..).collect::<Vec<_>>();
    //     tmp
    // }
}

#[derive(Component, Debug)]
pub struct CharacterState {
    last_serial: usize,
    pitch: f32,
    yaw: f32,
    up: Vec3,
    right: Vec3,
}

impl Default for CharacterState {
    fn default() -> Self {
        Self {
            last_serial: 0,
            pitch: 0.0,
            yaw: 0.0,
            up: Vec3::Y,
            right: Vec3::X,
        }
    }
}

impl CharacterState {
    pub fn rotation_full(&self) -> Quat {
        Quat::from_axis_angle(self.up, self.yaw.to_radians())
            * Quat::from_axis_angle(self.right, self.pitch.to_radians())
    }

    pub fn rotation_up(&self) -> Quat {
        Quat::from_axis_angle(self.up, self.yaw.to_radians())
    }

    pub fn forward_vec(rotation: &Quat) -> Vec3 {
        // at neutral rotation (Quat [0.0, 0.0, 0.0, 1.0] camera looks along negative Z axis, so this mean forward in out local coord system)
        rotation.mul_vec3(-Vec3::Z).normalize()
    }

    pub fn forward_on_groudplane(&self) -> Vec3 {
        // at neutral rotation (Quat [0.0, 0.0, 0.0, 1.0] camera looks along negative Z axis, so this mean forward in out local coord system)
        self.rotation_up().mul_vec3(-Vec3::Z).normalize()
    }

    pub fn right_on_groudplane(&self) -> Vec3 {
        self.rotation_up().mul_vec3(self.right).normalize()
    }
}

fn capture_input_state(
    time: Res<Time>,
    mut mouse_motion_event_reader: EventReader<MouseMotion>,
    input: ResMut<Input<KeyCode>>,
    mut queue: ResMut<InputStateQueue>,
    input_mapping: Res<InputMapping>,
) {
    let mut delta: Vec2 = Vec2::ZERO;
    for event in mouse_motion_event_reader.iter() {
        delta += event.delta;
    }
    if delta.is_nan() {
        return;
    }

    const SCALE: f32 = 0.25;
    debug!("send input state: {:?}", time);
    queue.push(InputState {
        time: time.clone(),
        serial: 0,
        delta_pitch: -delta.y * SCALE,
        delta_yaw: -delta.x * SCALE,
        forward: input_mapping.is_forward(&input),
        backward: input_mapping.is_backward(&input),
        strafe_right: input_mapping.is_strafe_right(&input),
        strafe_left: input_mapping.is_strafe_left(&input),
        walk: input_mapping.is_walk(&input),
    })
}

#[derive(Component)]
pub struct InputTarget;

fn apply_input_states(
    mut contact_debug: ResMut<ContactDebug>,
    // mut debug_lines: ResMut<debug_lines::DebugLines>,
    time: Res<Time>,
    mut crappify_timer: ResMut<Timer>,
    mut queue: ResMut<InputStateQueue>,
    mut query: Query<(&mut CharacterState, &mut Transform), With<InputTarget>>,
    query_pipeline: Res<QueryPipeline>,
    collider_query: QueryPipelineColliderComponentsQuery,
) {
    crappify_timer.tick(time.delta());
    // if !crappify_timer.just_finished() {
    //     return;
    // }

    for (mut character_state, mut transform) in query.iter_mut() {
        let mut trans_all = Vec3::ZERO;
        const WALK_SPEED: f32 = 2.0; // ms⁻¹
        const RUN_SPEED: f32 = 6.0; // ms⁻¹
        debug!("pending input states: {}", queue.len());
        for input_state in queue.iter() {
            character_state.last_serial = input_state.serial;
            character_state.yaw += input_state.delta_yaw;
            character_state.pitch += input_state.delta_pitch;

            let forward_vec = character_state.forward_on_groudplane();
            let right_vec = character_state.right_on_groudplane();

            debug!("forward: {:?} right: {:?}", forward_vec, right_vec);

            // let trans_start = trans;
            let dt = input_state.time.delta_seconds();
            let speed = if input_state.walk {
                WALK_SPEED
            } else {
                RUN_SPEED
            };
            let mut trans = Vec3::ZERO;

            if input_state.forward {
                trans += forward_vec * speed;
            }
            if input_state.backward {
                trans += forward_vec * -speed;
            }

            if input_state.strafe_right {
                trans += right_vec * speed;
            }
            if input_state.strafe_left {
                trans += right_vec * -speed;
            }

            trans = step_slidemove_try1(
                // &mut debug_lines,
                &mut contact_debug,
                &collider_query,
                transform.translation + trans_all,
                trans,
                dt,
                &query_pipeline,
            );
            trans_all += trans;
        }

        queue.retire_up_to(character_state.last_serial);
        transform.rotation = character_state.rotation_full();
        transform.translation += trans_all;
        debug!("{:?} {:?}", *character_state, transform.rotation);
    }
}

#[derive(Default)]
struct ContactDebug {
    add: Vec<(Contact, Vec3)>,
    add_pointer: Vec<(Vec3, Vec3)>,
    plane_mesh: Option<Handle<Mesh>>,
}

#[derive(Debug, Clone)]
struct Contact {
    collider_normal: Vec3,
    collider_point: Vec3,
    shape_normal: Vec3,
    shape_point: Vec3,
}

#[derive(Debug, Clone)]
enum CastResult {
    NoHit,
    Impact(f32, Contact),
    Touch(Contact),
    Stuck,
    Failed,
}
fn slidemove_none(
    contact_debug: &mut ContactDebug,
    collider_query: &QueryPipelineColliderComponentsQuery,
    origin: Vec3,
    velocity: Vec3,
    time: f32,
    query_pipeline: &Res<QueryPipeline>,
) -> Vec3 {
    let res = trace(collider_query, origin, velocity, query_pipeline, time);
    match &res {
        CastResult::Impact(toi, ref contact) => contact_debug
            .add
            .push((contact.clone(), origin + velocity * *toi)),
        CastResult::Touch(ref contact) => contact_debug.add.push((contact.clone(), origin)),
        _ => (),
    }

    match res {
        CastResult::NoHit => velocity * time,
        CastResult::Impact(toi, _) => velocity * toi,
        CastResult::Touch(_) | CastResult::Stuck | CastResult::Failed => Vec3::ZERO,
    }
}

// port of quake 3 PM_ClipVelocity
fn do_clip_velocity(v_in: Vec3, normal: Vec3, overbounce: f32) -> Vec3 {
    let backoff = match v_in.dot(normal) {
        dot if dot < 0.0 => dot * overbounce,
        dot => dot / overbounce,
    };
    let change = Vec3::new(normal.x * backoff, normal.y * backoff, normal.z * backoff);
    v_in - change
}

fn step_slidemove_try1(
    contact_debug: &mut ContactDebug,
    // debug_lines: &mut debug_lines::DebugLines,
    collider_query: &QueryPipelineColliderComponentsQuery,
    origin: Vec3,
    mut velocity: Vec3,
    mut time: f32,
    query_pipeline: &Res<QueryPipeline>,
) -> Vec3 {
    let (mut move_v, bump) = slidemove_try1(
        contact_debug,
        collider_query,
        origin,
        velocity,
        time,
        query_pipeline,
    );
    // info!("bump: {:?}", bump);
    if !bump {
        // goal reached without wall interaction -> done
        return move_v;
    }
    const STEP_SIZE: f32 = 0.11;
    let step_dir = -Vec3::Y;
    // groundtrace
    let res = trace(collider_query, origin, step_dir, query_pipeline, STEP_SIZE);
    info!("ground trace1: {:?}", res);
    let toi = match res {
        CastResult::NoHit | CastResult::Stuck | CastResult::Failed => {
            return move_v - step_dir * STEP_SIZE
        }
        CastResult::Impact(toi, _) => toi,
        CastResult::Touch(_) => 0.0,
    };

    let mut move_v = step_dir * toi * 0.99;

    // hop
    let step_dir = Vec3::Y;

    let res = trace(
        collider_query,
        origin + move_v,
        step_dir,
        query_pipeline,
        STEP_SIZE,
    );

    info!("hop: {:?}", res);

    let toi = match res {
        CastResult::Stuck | CastResult::Failed => return move_v,
        CastResult::NoHit => STEP_SIZE,
        CastResult::Impact(toi, _) => toi,
        CastResult::Touch(_) => 0.0,
    };

    move_v += step_dir * toi;

    let (move_v2, bump) = slidemove_try1(
        contact_debug,
        collider_query,
        origin + move_v,
        velocity,
        time,
        query_pipeline,
    );
    move_v += move_v2;

    // groundtrace
    let step_dir = -Vec3::Y;

    let res = trace(
        collider_query,
        origin + move_v,
        step_dir,
        query_pipeline,
        STEP_SIZE,
    );
    info!("ground trace2: {:?}", res);

    let toi = match res {
        CastResult::NoHit | CastResult::Stuck | CastResult::Failed => {
            return move_v - step_dir * STEP_SIZE
        }
        CastResult::Impact(toi, _) => toi,
        CastResult::Touch(_) => 0.0,
    };

    move_v += step_dir * toi * 0.99;

    move_v
}
fn slidemove_try1(
    contact_debug: &mut ContactDebug,
    // debug_lines: &mut debug_lines::DebugLines,
    collider_query: &QueryPipelineColliderComponentsQuery,
    origin: Vec3,
    mut velocity: Vec3,
    mut time: f32,
    query_pipeline: &Res<QueryPipeline>,
) -> (Vec3, bool) {
    let mut planes = Vec::new();
    let mut move_v = Vec3::ZERO;

    // TODO: gravity and ground trace
    // info!("slidemove");
    // initial velocity defines first clipping plane -> avoid to be nudged backwards (due to overclip?)
    planes.push(velocity.normalize());
    for bump in 0..4 {
        let res = trace(
            collider_query,
            origin + move_v,
            velocity,
            query_pipeline,
            time,
        );
        match &res {
            CastResult::Impact(toi, ref contact) => {
                contact_debug
                    .add
                    .push((contact.clone(), origin + velocity * *toi));
            }
            CastResult::Touch(ref contact) => {
                contact_debug.add.push((contact.clone(), origin));
            }
            _ => (),
        }
        if bump >= 2 {
            info!("bump: {}", bump);
        }
        // info!("res: {:?}", res);
        let (f, normal) = match res {
            CastResult::NoHit => return (velocity * time, bump != 0), // no intersection -> instantly accept whole move
            CastResult::Stuck | CastResult::Failed => return (Vec3::ZERO, false), // TODO: we probably need handling for being stuck (push back?)
            CastResult::Impact(toi, contact) => (toi, contact.collider_normal),
            CastResult::Touch(contact) => (0.0, contact.collider_normal),
        };
        contact_debug
            .add_pointer
            .push((origin + move_v, velocity * f));

        // accumulate movement and time-increments up to next intersection
        move_v += velocity * f;
        time -= time * f;

        // use contact normal as clip plane
        planes.push(normal);

        // actual clipping: try to make velocity parallel to all clip planes
        // find first plane we intersect
        for (i, plane) in planes.iter().enumerate() {
            let into = velocity.normalize().dot(*plane);
            if into >= 0.1 {
                continue; // move doesn't interact with the plane
            }
            // TODO: store impact speed

            // slide along the plane
            const OVERCLIP: f32 = 1.01;
            let mut clip_velocity = do_clip_velocity(velocity, *plane, OVERCLIP);
            // TODO: end velocity for gravity

            // find second plane
            for (j, plane2) in planes.iter().enumerate() {
                if j == i {
                    continue;
                }

                if clip_velocity.dot(*plane2) >= 0.1 {
                    continue;
                }

                // re-clip velocity with second plane
                clip_velocity = do_clip_velocity(clip_velocity, *plane2, OVERCLIP);
                if clip_velocity.dot(*plane) >= 0.0 {
                    continue;
                }

                // slide along the crease of the two planes (based on original velocity!)
                let dir = plane.cross(*plane2).normalize();

                let d = dir.dot(velocity);
                clip_velocity = dir * d;

                // is there a third plane we clip?
                for (k, plane3) in planes.iter().enumerate() {
                    if k == i || k == j {
                        continue;
                    }
                    if clip_velocity.dot(*plane3) >= 0.1 {
                        continue;
                    }

                    // give up on triple plane intersections
                    warn!("triple plane interaction");
                    return (Vec3::ZERO, false);
                }
            }
            // all interactions should be fixed -> try another move
            velocity = clip_velocity;
            break;
        }
    }
    (move_v, true)
}

fn trace(
    collider_query: &QueryPipelineColliderComponentsQuery,
    origin: Vec3,
    velocity: Vec3,
    query_pipeline: &Res<QueryPipeline>,
    time: f32,
) -> CastResult {
    let collider_set = QueryPipelineColliderComponentsSet(collider_query);
    let shape = Cylinder::new(0.9, 0.2);
    let shape_pos = Isometry::translation(origin.x, origin.y, origin.z);
    let shape_vel = velocity.into();
    let groups = InteractionGroups::all();
    let filter = None;
    if let Some((handle, hit)) = query_pipeline.cast_shape(
        &collider_set,
        &shape_pos,
        &shape_vel,
        &shape,
        time,
        groups,
        filter,
    ) {
        use bevy_rapier3d::rapier::parry::query::TOIStatus;

        let contact = Contact {
            collider_normal: (*hit.normal1).into(),
            collider_point: hit.witness1.into(),
            shape_normal: (*hit.normal2).into(),
            shape_point: hit.witness2.into(),
        };
        match hit.status {
            TOIStatus::Converged if hit.toi > 0.001 => CastResult::Impact(hit.toi, contact),
            TOIStatus::Converged => CastResult::Touch(contact),
            TOIStatus::Failed | TOIStatus::OutOfIterations => CastResult::Failed,
            TOIStatus::Penetrating => CastResult::Stuck,
        }
    } else {
        CastResult::NoHit
    }
}

fn slidemove_crap(
    collider_query: &QueryPipelineColliderComponentsQuery,
    origin: Vec3,
    velocity: Vec3,
    query_pipeline: &Res<QueryPipeline>,
) -> Vec3 {
    // ground trace
    // Wrap the bevy query so it can be used by the query pipeline.

    let velocity_start = velocity;
    let collider_set = QueryPipelineColliderComponentsSet(collider_query);
    // let shape = Cuboid::new(Vec3::new(0.2, 0.9, 0.2).into());
    let shape = Cylinder::new(0.9, 0.2);
    let mut shape_pos = Isometry::translation(origin.x, origin.y, origin.z);
    let shape_vel = (-Vec3::Y * 0.1).into();
    let max_toi = 4.0;
    let groups = InteractionGroups::all();
    let filter = None;

    let toi = if let Some((handle, hit)) = query_pipeline.cast_shape(
        &collider_set,
        &shape_pos,
        &shape_vel,
        &shape,
        max_toi,
        groups,
        filter,
    ) {
        // The first collider hit has the handle `handle`. The `hit` is a
        // structure containing details about the hit configuration.
        info!(
            "Hit the entity {:?} with the configuration: {:?}",
            handle.entity(),
            hit
        );
        hit.toi
    } else {
        1.0
    };
    info!("groundtrace1: {}", toi);
    let shape_pos_start = shape_pos;
    shape_pos.append_translation_mut(&(shape_vel * toi).into());
    shape_pos.append_translation_mut(&Vec3::new(0.0, 0.11, 0.0).into());
    let shape_vel = velocity.into();
    let toi = if let Some((handle, hit)) = query_pipeline.cast_shape(
        &collider_set,
        &shape_pos,
        &shape_vel,
        &shape,
        max_toi,
        groups,
        filter,
    ) {
        // The first collider hit has the handle `handle`. The `hit` is a
        // structure containing details about the hit configuration.
        info!(
            "Hit the entity {:?} with the configuration: {:?}",
            handle.entity(),
            hit
        );
        hit.toi
    } else {
        1.0
    };
    info!("forward: {}", toi);
    shape_pos.append_translation_mut(&(shape_vel * toi).into());
    let shape_vel = (-Vec3::Y * 0.2).into();
    let toi = if let Some((handle, hit)) = query_pipeline.cast_shape(
        &collider_set,
        &shape_pos,
        &shape_vel,
        &shape,
        max_toi,
        groups,
        filter,
    ) {
        // The first collider hit has the handle `handle`. The `hit` is a
        // structure containing details about the hit configuration.
        info!(
            "Hit the entity {:?} with the configuration: {:?}",
            handle.entity(),
            hit
        );
        hit.toi
    } else {
        1.0
    };
    info!("groundtrace2: {}", toi);
    shape_pos.append_translation_mut(&(shape_vel * toi).into());
    let d: Vec3 = (shape_pos.translation.vector - shape_pos_start.translation.vector).into();
    velocity_start + d
}

#[derive(Component)]
struct ContactDebugMesh {
    elapsed: Timer,
}

fn contact_debug(
    time: Res<Time>,
    mut commands: Commands,
    mut contact_debug: ResMut<ContactDebug>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut reaper_query: Query<(Entity, &mut ContactDebugMesh)>,
    mut debug_lines: ResMut<debug_lines::DebugLines>,
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

#[derive(Default)]
pub struct CharacterStateInputPlugin;

impl Plugin for CharacterStateInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(capture_input_state)
            .add_system(apply_input_states)
            .add_system(contact_debug)
            .insert_resource(InputMapping::default())
            .insert_resource(Timer::from_seconds(0.5, true))
            .insert_resource(InputStateQueue::default())
            .insert_resource(ContactDebug::default());
    }
}
