use std::collections::VecDeque;

use bevy::input::keyboard::KeyboardInput;
use bevy::input::mouse::MouseMotion;
use bevy::math::Vec3;
use bevy::prelude::*;
use bevy::utils::Instant;
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

    const SCALE: f32 = 0.5;
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
        let mut trans = Vec3::ZERO;
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

            let trans_start = trans;
            let dt = input_state.time.delta_seconds();
            let speed = if input_state.walk {
                WALK_SPEED
            } else {
                RUN_SPEED
            };

            if input_state.forward {
                trans += forward_vec * speed * dt;
            }
            if input_state.backward {
                trans += forward_vec * -speed * dt;
            }

            if input_state.strafe_right {
                trans += right_vec * speed * dt;
            }
            if input_state.strafe_left {
                trans += right_vec * -speed * dt;
            }

            // ground trace

            // Wrap the bevy query so it can be used by the query pipeline.
            let collider_set = QueryPipelineColliderComponentsSet(&collider_query);

            let shape = Cuboid::new(Vec3::new(0.2, 0.9, 0.2).into());
            let mut shape_pos = transform.translation.into();
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

            let shape_vel = trans.into();

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

            let d: Vec3 =
                (shape_pos.translation.vector - shape_pos_start.translation.vector).into();
            trans = trans_start + d;
        }

        queue.retire_up_to(character_state.last_serial);
        transform.rotation = character_state.rotation_full();
        transform.translation += trans;
        debug!("{:?} {:?}", *character_state, transform.rotation);
    }
}

#[derive(Default)]
pub struct CharacterStateInputPlugin;

impl Plugin for CharacterStateInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(capture_input_state)
            .add_system(apply_input_states)
            .insert_resource(InputMapping::default())
            .insert_resource(Timer::from_seconds(0.5, true))
            .insert_resource(InputStateQueue::default());
    }
}
