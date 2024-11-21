use std::collections::VecDeque;

use bevy::{input::mouse::MouseMotion, prelude::*};
use bevy_rapier3d::{parry::simba::scalar::SupersetOf, prelude::*};

use shared::AppState;

#[derive(Component, Default, Debug)]
pub struct PlayerState {
    pub last_applied_serial: Option<u32>,
    pub lon: f32,
    pub lat: f32,

    // pub z_velocity: f32,
    pub velocity: Vec3,

    pub last_jump: f32,

    pub rotation: Quat,

    pub gravity: Option<f32>,
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
    pub jump: bool,

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

    pub jump: KeyCode,

    pub walk: KeyCode,
}

impl Default for PlayerInputSource {
    fn default() -> Self {
        Self {
            next_serial: 0,
            forward: KeyCode::KeyW,
            backward: KeyCode::KeyS,
            left: KeyCode::KeyA,
            right: KeyCode::KeyD,
            up: KeyCode::KeyR,
            down: KeyCode::KeyF,
            jump: KeyCode::Space,
            walk: KeyCode::ShiftLeft,
        }
    }
}

#[derive(Component, Default, Debug)]
pub struct PlayerCamera;

pub fn apply_ground_friction(v: Vec2, decel: f32) -> Vec2 {
    let vnorm = v.normalize_or_zero();
    let len = v.length();
    if len > decel {
        vnorm * (len - decel)
    } else {
        Vec2::ZERO
    }
}

pub fn player_controller_input_system(
    key_codes: Res<ButtonInput<KeyCode>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut query: Query<(&mut PlayerInputSource, &mut PlayerInputQueue)>,
    app_state: Res<State<AppState>>,
) {
    for (mut input_source, mut queue) in &mut query {
        let input_enabled =
            *app_state.get() != AppState::Editor || key_codes.pressed(input_source.walk);

        if !input_enabled {
            continue;
        }

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

        const WALK_SPEED: f32 = 2.0;
        const RUN_SPEED: f32 = 6.0;

        let speed = if key_codes.pressed(input_source.walk) {
            WALK_SPEED
        } else {
            RUN_SPEED
        };
        forward *= speed;
        right *= speed;

        let mut lon_raw = 0.0;
        let mut lat_raw = 0.0;

        for event in mouse_motion.read() {
            lon_raw -= event.delta.x; // * SENSITIVITY;
            lat_raw -= event.delta.y; // * SENSITIVITY;
        }

        const SENS: f32 = 0.05;
        let mut lon = lon_raw * SENS;
        let mut lat = lat_raw * SENS;

        let accel = [(0.5..2.0, 2.0), (2.0..f32::MAX, 3.0)];
        for a in &accel {
            if a.0.contains(&lon.abs()) {
                lon *= a.1
            }
            if a.0.contains(&lat.abs()) {
                lat *= a.1
            }
        }
        let jump = key_codes.pressed(input_source.jump);
        // info!("input: {} {} -> {} {}", lon_raw, lat_raw, lon, lat);
        let player_input = PlayerInput {
            serial: input_source.next_serial,
            forward,
            right,
            up,
            jump,
            lon,
            lat,
        };

        input_source.next_serial += 1;
        queue.queue.push_back(player_input);
    }
}

pub fn hack_toggle_gravity_system(
    key_codes: Res<ButtonInput<KeyCode>>,

    mut query: Query<&mut PlayerState>,
) {
    if key_codes.just_released(KeyCode::KeyQ) {
        for mut player_state in &mut query {
            if player_state.gravity.is_none() {
                player_state.gravity = Some(-9.81);
            } else {
                player_state.gravity = None;
            }
        }
    }
}
pub fn player_controller_apply_system(
    time: Res<Time>,
    mut query: Query<(
        &mut Transform,
        &mut KinematicCharacterController,
        &mut PlayerState,
        &mut PlayerInputQueue,
        Option<&KinematicCharacterControllerOutput>,
    )>,
) {
    for (mut _transform, mut character_controller, mut player_state, mut input_queue, output) in
        &mut query
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
            // if input.lon.abs() > 0.0 || input.lat.abs() > 0.0 {
            //     info!("apply: {} {}", input.lon, input.lat);
            // }
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
            let dt = time.delta_seconds();
            // const DT: f32 = 1.0 / 60.0; // fixed timestep

            let forward = y_rot * (-Vec3::Z * input.forward);
            let right = y_rot * (Vec3::X * input.right);

            if forward.length() != 0.0 || right.length() != 0.0 {
                player_state.velocity.x = forward.x + right.x;
                player_state.velocity.z = forward.z + right.z;
            } else {
                const DECEL: f32 = 30.0;
                let Vec2 { x, y: z } =
                    apply_ground_friction(player_state.velocity.xz(), DECEL * dt);
                player_state.velocity.x = x;
                player_state.velocity.z = z;
            }
            // * character_controller.custom_mass.unwrap_or(1.0);

            // info!("{:?} {:?}", forward, right);
            // transform.translation += forward;
            // transform.translation += right;
            let up = Vec3::ZERO;
            // character_controller.max_slope_climb_angle = std::f32::consts::PI / 2.0;
            let mut up = Vec3::ZERO;
            if let Some(gravity) = player_state.gravity {
                if let Some(output) = output {
                    if output.grounded {
                        if input.jump && time.elapsed_seconds() - player_state.last_jump >= 0.5 {
                            player_state.velocity.y = 4.0;
                            player_state.last_jump = time.elapsed_seconds();
                            info!("jump");
                        } else if time.elapsed_seconds() - player_state.last_jump >= 0.1 {
                            player_state.velocity.y = 0.0;
                        }
                    } else {
                        player_state.velocity.y += gravity * 2.0 * dt;
                    }
                }
            } else {
                player_state.velocity.y = input.up * dt;
            }

            if let Some(output) = output {
                info!(
                    "grounded: {}\tsliding: {}",
                    output.grounded, output.is_sliding_down_slope
                );
            }
            player_state.velocity.y += up.y;
            // player_state.velocity.y += Vec3::Y * player_state.z_velocity * dt;
            // let up = if let Some(gravity) = player_state.gravity {

            // }

            character_controller.translation = Some(player_state.velocity * dt);
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

        debug!("collisions: {:?}", controller_output.collisions);

        // for c in &cko.collisions {
        //     info!("{:?}", c.toi.normal2);
        // }
    }
}

fn sync_player_camera_system(
    player_query: Query<(&Transform, &PlayerState), Without<PlayerCamera>>,
    mut camera_query: Query<&mut Transform, With<PlayerCamera>>,
) {
    let Ok((player, player_state)) = player_query.get_single() else {
        return;
    };
    let Ok(mut camera) = camera_query.get_single_mut() else {
        return;
    };

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
            character_controller: KinematicCharacterController {
                custom_mass: Some(5.0),
                up: Vec3::Y,
                offset: CharacterLength::Absolute(0.1),
                slide: true,
                autostep: Some(CharacterAutostep {
                    max_height: CharacterLength::Absolute(0.3),
                    min_width: CharacterLength::Relative(0.5),
                    include_dynamic_bodies: false,
                }),
                max_slope_climb_angle: 40.0f32.to_radians(),
                min_slope_slide_angle: 30.0f32.to_radians(),
                apply_impulse_to_dynamic_bodies: true,
                snap_to_ground: Some(CharacterLength::Absolute(0.2)),
                ..default()
            },
            input_queue: default(),
            input_source: default(),
            // collider: Collider::cylinder(0.9, 0.3),
            collider: Collider::cuboid(0.3, 0.9, 0.3),
        }
    }
}

pub struct PlayerControllerPlugin;

impl Plugin for PlayerControllerPlugin {
    fn build(&self, app: &mut App) {
        // app.add_system_set(
        //     SystemSet::on_update(AppState::InGame).with_system(player_controller_input_system),
        // );
        app.add_systems(Update, player_controller_input_system);
        app.add_systems(
            Update,
            player_controller_apply_system.after(player_controller_input_system),
        )
        .add_systems(
            Update,
            player_controller_apply_output_system.before(player_controller_apply_system),
        )
        .add_systems(
            Update,
            sync_player_camera_system.after(player_controller_apply_output_system),
        )
        .add_systems(Update, hack_toggle_gravity_system);
    }
}
