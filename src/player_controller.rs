use std::collections::VecDeque;

use bevy::{input::mouse::MouseMotion, prelude::*};
use bevy_rapier3d::prelude::*;

use crate::AppState;

#[derive(Component, Default, Debug)]
pub struct PlayerState {
    pub last_applied_serial: Option<u32>,
    pub lon: f32,
    pub lat: f32,

    pub rotation: Quat,
}

impl PlayerState {
    pub fn get_y_rotation(&self) -> Quat {
        Quat::from_axis_angle(Vec3::Y, self.lon.to_radians())
    }
    pub fn get_rotation(&self) -> Quat {
        self.get_y_rotation() * Quat::from_axis_angle(Vec3::X, self.lat.to_radians())
    }
}

#[derive(Component, Default, Debug)]
pub struct PlayerInput {
    pub serial: u32,
    pub forward: f32,
    pub right: f32,
    pub up: f32,

    pub lon: f32,
    pub lat: f32,
}

#[derive(Component, Default, Debug)]
pub struct PlayerInputQueue {
    pub queue: VecDeque<PlayerInput>,
}

#[derive(Component, Debug)]
pub struct PlayerInputSource {
    pub next_serial: u32,

    // key bindings
    pub forward: KeyCode,
    pub backward: KeyCode,
    pub left: KeyCode,
    pub right: KeyCode,

    pub up: KeyCode,
    pub down: KeyCode,
}

impl Default for PlayerInputSource {
    fn default() -> Self {
        Self {
            next_serial: 0,
            forward: KeyCode::W,
            backward: KeyCode::S,
            left: KeyCode::A,
            right: KeyCode::D,
            up: KeyCode::R,
            down: KeyCode::F,
        }
    }
}

#[derive(Component, Default, Debug)]
pub struct PlayerCamera;

pub fn player_controller_input_system(
    key_codes: Res<Input<KeyCode>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut query: Query<(&mut PlayerInputSource, &mut PlayerInputQueue)>,
) {
    for (mut input_source, mut queue) in &mut query {
        let mut forward = 0.0;
        let mut right = 0.0;
        let mut up = 0.0;
        if key_codes.pressed(input_source.forward) {
            forward += 1.0;
        }
        if key_codes.pressed(input_source.backward) {
            forward -= 1.0;
        }
        if key_codes.pressed(input_source.left) {
            right -= 1.0;
        }
        if key_codes.pressed(input_source.right) {
            right += 1.0;
        }

        if key_codes.pressed(input_source.up) {
            up += 1.0;
        }
        if key_codes.pressed(input_source.down) {
            up -= 1.0;
        }
        const WALK_SPEED: f32 = 6.0;
        forward *= WALK_SPEED;
        right *= WALK_SPEED;

        let mut lon = 0.0;
        let mut lat = 0.0;

        for event in mouse_motion.iter() {
            const SENSITIVITY: f32 = 0.5;
            lon -= event.delta.x * SENSITIVITY;
            lat -= event.delta.y * SENSITIVITY;
        }

        let player_input = PlayerInput {
            serial: input_source.next_serial,
            forward,
            right,
            up,
            lon,
            lat,
        };

        input_source.next_serial += 1;
        queue.queue.push_back(player_input);
    }
}

pub fn player_controller_apply_system(
    mut query: Query<(
        &mut Transform,
        &mut KinematicCharacterController,
        &mut PlayerState,
        &mut PlayerInputQueue,
    )>,
) {
    for (mut _transform, mut character_controller, mut player_state, mut input_queue) in &mut query
    {
        // let (discard, apply) = if let Some(last_applied) = player_state.last_applied_serial {
        //     if let Some((first_newer, _)) = input_queue
        //         .queue
        //         .iter()
        //         .enumerate()
        //         .find(|(i, input)| input.serial > last_applied)
        //     {
        //         (0..first_newer, first_newer..input_queue.queue.len())
        //     } else {
        //         // no newer input in the queue -> do nothing
        //         (0..0, 0..0)
        //     }
        // } else {
        //     (0..0, 0..input_queue.queue.len())
        // };

        // for input in input_queue.queue.drain(apply) {}

        // input_queue.queue.drain(discard);

        // FIXME: we can only apply one user input per frame due to KinematicCharacterController design
        while input_queue.queue.len() > 1 {
            info!("skipping user input: {:?}", input_queue.queue.front());
            input_queue.queue.pop_front().unwrap();
        }

        if let Some(input) = input_queue.queue.pop_front() {
            // FIXME: this wants to be a let chain...
            match player_state.last_applied_serial {
                Some(serial) if serial >= input.serial => {
                    info!("input already applied!? {}", input.serial);
                    continue;
                }
                _ => (),
            }

            player_state.last_applied_serial = Some(input.serial);
            player_state.lon += input.lon;
            player_state.lat += input.lat;

            player_state.lat = player_state.lat.clamp(-85.0, 85.0);
            while player_state.lon < 0.0 {
                player_state.lon += 360.0;
            }
            while player_state.lon >= 360.0 {
                player_state.lon -= 360.0;
            }

            let y_rot = player_state.get_y_rotation();
            const DT: f32 = 1.0 / 60.0; // fixed timestep

            let forward = y_rot * (-Vec3::Z * input.forward) * DT;
            let right = y_rot * (Vec3::X * input.right) * DT;
            let up = input.up * Vec3::Y * DT;

            // info!("{:?} {:?}", forward, right);
            // transform.translation += forward;
            // transform.translation += right;
            character_controller.max_slope_climb_angle = std::f32::consts::PI / 2.0;
            character_controller.translation = Some(forward + right + up);
            debug!("want: {:?}", character_controller.translation);
            character_controller.autostep = Some(CharacterAutostep::default());
        }

        assert!(input_queue.queue.is_empty());
    }
}

fn player_controller_apply_output_system(
    mut query: Query<
        (
            &mut Transform,
            &KinematicCharacterController,
            &KinematicCharacterControllerOutput,
        ),
        With<PlayerState>,
    >,
) {
    for (mut transform, _ck, controller_output) in &mut query {
        // info!("{cko:?}");
        transform.translation += controller_output.effective_translation;
        debug!("got: {:?}", controller_output.effective_translation);

        // info!("collisions: {:?}", cko.collisions);

        // for c in &cko.collisions {
        //     info!("{:?}", c.toi.normal2);
        // }
    }
}

fn sync_player_camera_system(
    player_query: Query<(&Transform, &PlayerState), Without<PlayerCamera>>,
    mut camera_query: Query<&mut Transform, With<PlayerCamera>>,
) {
    let Ok((player, player_state)) = player_query.get_single() else { return };
    let Ok(mut camera) = camera_query.get_single_mut() else { return };

    camera.translation = player.translation + Vec3::Y * 0.85;
    camera.rotation = player_state.get_rotation();
}

#[derive(Bundle)]
pub struct PlayerControllerBundle {
    player_state: PlayerState,
    character_controller: KinematicCharacterController,
    input_queue: PlayerInputQueue,
    input_source: PlayerInputSource,
    collider: Collider,
}

impl Default for PlayerControllerBundle {
    fn default() -> Self {
        Self {
            // collider: Collider::cuboid(0.2, 0.9, 0.2),
            player_state: default(),
            character_controller: default(),
            input_queue: default(),
            input_source: default(),
            collider: Collider::cylinder(0.9, 0.3),
        }
    }
}

pub struct PlayerControllerPlugin;

impl Plugin for PlayerControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(
            SystemSet::on_update(AppState::InGame).with_system(player_controller_input_system),
        )
        .add_system(player_controller_apply_system.after(player_controller_input_system))
        .add_system(player_controller_apply_output_system.before(player_controller_apply_system))
        .add_system(sync_player_camera_system.after(player_controller_apply_output_system));
    }
}
