use bevy::{input::mouse::MouseMotion, prelude::*};
use bevy_rapier3d::{na::Quaternion, prelude::*};

#[derive(Component, Default, Debug)]
pub struct PlayerState {
    pub lon: f32,
    pub lat: f32,

    pub forward: f32,
    pub right: f32,
    pub up: f32,

    pub rotation: Quat,
}

#[derive(Component, Default, Debug)]
pub struct PlayerCamera;

pub fn player_controller_input_system(
    key_codes: Res<Input<KeyCode>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut query: Query<&mut PlayerState>,
) {
    for mut player_state in &mut query {
        let w = key_codes.pressed(KeyCode::W);
        let a = key_codes.pressed(KeyCode::A);
        let s = key_codes.pressed(KeyCode::S);
        let d = key_codes.pressed(KeyCode::D);

        let r = key_codes.pressed(KeyCode::R);
        let f = key_codes.pressed(KeyCode::F);

        let mut forward = 0.0;
        let mut right = 0.0;
        let mut up = 0.0;
        if w {
            forward += 1.0;
        }
        if s {
            forward -= 1.0;
        }
        if a {
            right -= 1.0;
        }
        if d {
            right += 1.0;
        }

        if r {
            up += 1.0;
        }
        if f {
            up -= 1.0;
        }
        const WALK_SPEED: f32 = 6.0 / 60.0;
        player_state.forward = forward * WALK_SPEED;
        player_state.right = right * WALK_SPEED;
        player_state.up = up * WALK_SPEED;

        for event in mouse_motion.iter() {
            const SENSITIVITY: f32 = 0.5;
            player_state.lon -= event.delta.x * SENSITIVITY;
            player_state.lat -= event.delta.y * SENSITIVITY;
        }
        player_state.lat = player_state.lat.clamp(-85.0, 85.0);
        while player_state.lon < 0.0 {
            player_state.lon += 360.0;
        }
        while player_state.lon >= 360.0 {
            player_state.lon -= 360.0;
        }
    }
}

pub fn player_controller_apply_system(
    mut query: Query<(
        &mut Transform,
        &mut KinematicCharacterController,
        &mut PlayerState,
    )>,
) {
    for (mut transform, mut character_controller, mut player_state) in &mut query {
        let y_rot = Quat::from_axis_angle(Vec3::Y, player_state.lon.to_radians());
        player_state.rotation =
            y_rot * Quat::from_axis_angle(Vec3::X, player_state.lat.to_radians());

        let forward = y_rot * (-Vec3::Z * player_state.forward);
        let right = y_rot * (Vec3::X * player_state.right);
        let up = player_state.up * Vec3::Y;

        // info!("{:?} {:?}", forward, right);
        // transform.translation += forward;
        // transform.translation += right;
        character_controller.max_slope_climb_angle = std::f32::consts::PI / 2.0;
        character_controller.translation = Some(forward + right + up);
        info!("want: {:?}", character_controller.translation);
        character_controller.autostep = Some(CharacterAutostep::default());
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
        info!("got: {:?}", controller_output.effective_translation);

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
    camera.rotation = player_state.rotation;
}

#[derive(Bundle, Default)]
pub struct PlayerControllerBundle {
    player_state: PlayerState,
    character_controller: KinematicCharacterController,
}

pub struct PlayerControllerPlugin;

impl Plugin for PlayerControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(player_controller_input_system)
            .add_system(player_controller_apply_system.after(player_controller_input_system))
            .add_system(
                player_controller_apply_output_system.before(player_controller_apply_system),
            )
            .add_system(sync_player_camera_system.after(player_controller_apply_output_system));
    }
}
