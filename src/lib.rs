use std::{collections::VecDeque, time::Duration};

use bevy::{input::mouse::MouseMotion, math::Vec3, prelude::*, render::mesh};
// use bevy_rapier3d::physics::{
//     QueryPipelineColliderComponentsQuery, QueryPipelineColliderComponentsSet,
// };
use bevy_rapier3d::prelude::*;

pub mod debug_lines;
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
        const WALK_SPEED: f32 = 0.5; // ms⁻¹
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

            trans = slidemove_try1(
                // &mut debug_lines,
                &mut contact_debug,
                &collider_query,
                transform.translation + trans_all,
                trans,
                dt,
                &query_pipeline,
            )
            .0;
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
    // Touch(Contact),
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
    let res = trace2(collider_query, origin, velocity * time, query_pipeline);

    if res.stuck {
        info!("stuck!");
        return Vec3::ZERO;
    }

    res.dist
    // match &res {
    //     CastResult::Impact(toi, ref contact) => contact_debug
    //         .add
    //         .push((contact.clone(), origin + velocity * *toi)),
    //     // CastResult::Touch(ref contact) => contact_debug.add.push((contact.clone(), origin)),
    //     _ => (),
    // }

    // match res {
    //     CastResult::NoHit => velocity * time,
    //     CastResult::Impact(toi, _) => {
    //         info!("impact: {}", toi);
    //         velocity * toi
    //     }
    //     CastResult::Stuck => {
    //         info!("stuck!");
    //         Vec3::ZERO
    //     }
    //     CastResult::Failed => Vec3::ZERO,
    // }
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
        // CastResult::Touch(_) => 0.0,
    };

    let mut move_v = step_dir * toi; // * 0.99;

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
        // CastResult::Touch(_) => 0.0,
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
        // CastResult::Touch(_) => 0.0,
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
    planes.push(Vec3::Y);

    info!("start");
    'bump: for bump in 0..4 {
        let trace_start_pos = origin + move_v;
        let trace_dist = velocity * time;

        let res = trace2(collider_query, trace_start_pos, trace_dist, query_pipeline);
        if res.stuck {
            error!("stuck!");
            return (Vec3::ZERO, true);
        }
        // info!(
        //     "bump {} {:?} {} {:?} {}",
        //     bump, trace_dist, res.f, velocity, time
        // );

        if let Some(contact) = res.contact {
            contact_debug
                .add
                .push((contact.clone(), trace_start_pos + res.dist));

            // use contact normal as clip plane
            move_v += res.dist;
            time -= time * res.f;

            //
            // if this is the same plane we hit before, nudge velocity
            // out along it, which fixes some epsilon issues with
            // non-axial planes
            //
            for plane in planes.iter() {
                let dot = contact.collider_normal.dot(*plane);
                if dot > 0.99 {
                    info!("dot: {} {:?} {:?}", dot, contact.collider_normal, plane);

                    velocity += contact.collider_normal;
                    info!("nudge");
                    continue 'bump;
                }
                info!("dot: {} {:?} {:?}", dot, contact.collider_normal, plane);
            }
            planes.push(contact.collider_normal);
        } else {
            info!("done");
            return (move_v + res.dist, false);
        }

        // if bump >= 1 {
        //     info!("bump: {}", bump);
        // }

        // actual clipping: try to make velocity parallel to all clip planes
        // find first plane we intersect
        for (i, plane) in planes.iter().enumerate() {
            let into = velocity.normalize().dot(*plane);
            if into >= 0.1 {
                continue; // move doesn't interact with the plane
            }
            // TODO: store impact speed

            // slide along the plane
            const OVERCLIP: f32 = 1.001;
            let mut clip_velocity = do_clip_velocity(velocity, *plane, OVERCLIP);
            // TODO: end velocity for gravity
            info!(
                "clip velocity1 {:?} {:?} {:?} {} {}",
                velocity, clip_velocity, plane, i, into
            );
            // find second plane
            for (j, plane2) in planes.iter().enumerate() {
                if j == i {
                    continue;
                }

                if clip_velocity.dot(*plane2) >= 0.1 {
                    continue;
                }

                // re-clip velocity with second plane
                let xc = clip_velocity;
                clip_velocity = do_clip_velocity(clip_velocity, *plane2, OVERCLIP);
                if clip_velocity.dot(*plane) >= 0.0 {
                    info!(
                        "clip velocity1.5 {:?} {:?} {:?} {}",
                        xc, clip_velocity, plane2, j
                    );

                    continue;
                }
                // info!(
                //     "clip velocity2 {:?} {:?} {:?} {}",
                //     xc, clip_velocity, plane2, j
                // );

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
                    warn!(
                        "triple plane interaction {:?} {:?} {:?}",
                        plane, plane2, plane3
                    );
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
            TOIStatus::Converged if hit.toi > 0.01 => CastResult::Impact(hit.toi * 0.99, contact),
            TOIStatus::Converged => CastResult::Impact(0.0, contact),
            // TOIStatus::Converged => CastResult::Touch(contact),
            TOIStatus::Failed | TOIStatus::OutOfIterations => CastResult::Failed,
            TOIStatus::Penetrating => CastResult::Stuck,
        }
    } else {
        CastResult::NoHit
    }
}

struct TraceResult {
    contact: Option<Contact>,
    stuck: bool,
    dist: Vec3,
    f: f32,
}

fn trace2(
    collider_query: &QueryPipelineColliderComponentsQuery,
    start: Vec3,
    dist: Vec3,
    query_pipeline: &Res<QueryPipeline>,
) -> TraceResult {
    let collider_set = QueryPipelineColliderComponentsSet(collider_query);
    let shape = Cylinder::new(0.9, 0.2);
    // let shape = Cuboid::new(Vec3::new(0.2, 0.9, 0.2).into());

    let shape_pos = Isometry::translation(start.x, start.y, start.z);
    let shape_vel = dist.into();
    let groups = InteractionGroups::all();
    let filter = None;
    let trace_result = if let Some((handle, hit)) = query_pipeline.cast_shape(
        &collider_set,
        &shape_pos,
        &shape_vel,
        &shape,
        1.0,
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
            TOIStatus::Converged if hit.toi > 0.05 => TraceResult {
                contact: Some(contact),
                dist: dist * hit.toi,
                stuck: false,
                f: hit.toi,
            },
            TOIStatus::Converged | TOIStatus::Failed | TOIStatus::OutOfIterations => TraceResult {
                contact: Some(contact),
                dist: Vec3::ZERO,
                stuck: false,
                f: 0.0,
            },
            TOIStatus::Penetrating => TraceResult {
                contact: None,
                dist: Vec3::ZERO,
                stuck: true,
                f: 0.0,
            },
        }
    } else {
        TraceResult {
            contact: None,
            dist: dist * 0.99,
            stuck: false,
            f: 0.99,
        }
    };

    let end_pos = start + trace_result.dist;
    let intersection = query_pipeline.intersection_with_shape(
        &collider_set,
        &end_pos.into(),
        &shape,
        groups,
        filter,
    );
    if let Some(_collider) = intersection {
        warn!(
            "trace ends up stuck. {:?} {} {} {}",
            trace_result.dist,
            trace_result.f,
            dist,
            dist.length()
        );
        return TraceResult {
            contact: None,
            dist: Vec3::ZERO,
            stuck: false,
            f: 0.0,
        };
    }

    trace_result
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
