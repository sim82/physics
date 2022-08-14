use bevy::{
    input::{keyboard::KeyboardInput, mouse::MouseMotion},
    math::Vec3,
    prelude::*,
};
use bevy_rapier3d::prelude::*;
use contact_debug::ContactDebug;
use std::collections::VecDeque;
use trace::{CollisionTraceable, TraceContact};

pub mod contact_debug;
// pub mod debug_lines;
pub mod slidemove;
pub mod trace;
pub const OVERCLIP: f32 = 1.001;

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
                bitmap.extend(
                    [
                        (l.max(r - d)).clamp(0, 255) as u8,
                        (l.max(g - d)).clamp(0, 255) as u8,
                        (l.max(b - d)).clamp(0, 255) as u8,
                        0u8,
                    ]
                    .iter(),
                );
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

pub struct InputState {
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

#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
pub struct CharacterState {
    last_serial: usize,
    pitch: f32,
    yaw: f32,
    up: Vec3,
    right: Vec3,
    velocity: Vec3,
}

impl Default for CharacterState {
    fn default() -> Self {
        Self {
            last_serial: 0,
            pitch: 0.0,
            yaw: 0.0,
            up: Vec3::Y,
            right: Vec3::X,
            velocity: Vec3::ZERO,
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

    pub fn ground_trace(
        &mut self,
        translation: Vec3,
        collision_system: &dyn CollisionTraceable,
    ) -> Option<TraceContact> {
        let down = -Vec3::Y * 0.02;
        let res = collision_system.trace2(translation, down);
        if res.f < 1.0 {
            // info!("on gound");
        }
        res.contact
    }

    pub fn apply_friction(&mut self, time: f32) {
        let vel = self.velocity;
        let speed = vel.length();
        if speed <= 0.0 {
            return;
        }
        const FRICTION: f32 = 10.0;
        let drop = FRICTION * time;
        let newspeed = f32::max(speed - drop, 0.0);
        self.velocity *= newspeed / speed;
        // info!("friction: {:?}", self.velocity);
    }

    pub fn apply_acceleration(&mut self, wish_velocity: Vec3, time: f32, accel: f32) {
        let push_v = wish_velocity - self.velocity;
        let push_len = push_v.length(); // TODO: combine?
        if push_len == 0.0 {
            return;
        }
        let push_dir = push_v.normalize();
        let wish_speed = wish_velocity.length();

        let can_push = accel * time * wish_speed;
        let can_push = if can_push > push_len {
            push_len
        } else {
            can_push
        };
        debug!(
            "apply acceleration: {} {} {}",
            self.velocity, can_push, push_dir
        );
        self.velocity += can_push * push_dir;
    }

    pub fn apply_user_input(
        &mut self,
        input_state: &InputState,
        translation: Vec3,
        collision_system: &dyn CollisionTraceable,
    ) -> Vec3 {
        const WALK_SPEED: f32 = 2.0; // ms⁻¹
        const RUN_SPEED: f32 = 6.0; // ms⁻¹

        self.last_serial = input_state.serial;
        self.yaw = modulo_range(self.yaw + input_state.delta_yaw, 360.0);
        self.pitch = modulo_range(self.pitch + input_state.delta_pitch, 360.0);

        let forward_vec = self.forward_on_groudplane();
        let right_vec = self.right_on_groudplane();

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
        let ground_contact = self.ground_trace(translation, collision_system);
        self.apply_friction(dt);
        self.apply_acceleration(trans, dt, 10.0);

        if let Some(ground_contact) = ground_contact {
            let old_velocity = self.velocity;
            self.velocity = slidemove::do_clip_velocity(
                self.velocity,
                ground_contact.collider_normal,
                OVERCLIP,
            );
            debug!("ground clip {} -> {}", old_velocity, self.velocity);
        }

        let (trans, new_velocity, _clip) =
            slidemove::slidemove_try2(collision_system, translation, self.velocity, dt);
        self.velocity = new_velocity;
        trans
    }
}

fn capture_input_state(
    time: Res<Time>,
    mut mouse_motion_event_reader: EventReader<MouseMotion>,
    input: ResMut<Input<KeyCode>>,
    mut queue: ResMut<InputStateQueue>,
    input_mapping: Res<InputMapping>,
    mouse_input_state: Res<MouseInputState>,
) {
    let mut delta: Vec2 = Vec2::ZERO;
    if mouse_input_state.use_mouse_input {
        for event in mouse_motion_event_reader.iter() {
            delta += event.delta;
        }
        if delta.is_nan() {
            return;
        }
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

fn modulo_range(mut v: f32, max: f32) -> f32 {
    while v > max {
        v -= max;
    }
    while v < 0.0 {
        v += max;
    }
    v
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
    // query_pipeline: Res<QueryPipeline>,
    // collider_query: QueryPipelineColliderComponentsQuery,
    rapier_context: Res<RapierContext>,
    mut debug_lines: ResMut<bevy_prototype_debug_lines::DebugLines>,
) {
    crappify_timer.tick(time.delta());
    // if !crappify_timer.just_finished() {
    //     return;
    // }

    for (mut character_state, mut transform) in query.iter_mut() {
        let mut trans_all = Vec3::ZERO;
        debug!("pending input states: {}", queue.len());
        for input_state in queue.iter() {
            // let collision_system = CollisionSystem {
            //     query_pipeline: &query_pipeline,
            //     collider_query: &collider_query,
            //     contact_debug: &mut contact_debug,
            // };
            let trans = character_state.apply_user_input(
                input_state,
                transform.translation + trans_all,
                &*rapier_context,
            );
            trans_all += trans;
        }
        debug_lines.line(
            transform.translation,
            transform.translation + trans_all,
            5.0,
        );

        queue.retire_up_to(character_state.last_serial);
        transform.rotation = character_state.rotation_full();
        transform.translation += trans_all;
        debug!("{:?} {:?}", *character_state, transform.rotation);
    }
}

pub struct MouseInputState {
    use_mouse_input: bool,
}

impl Default for MouseInputState {
    fn default() -> Self {
        Self {
            use_mouse_input: true,
        }
    }
}

fn update_mouse_input_state(
    mut mouse_input_state: ResMut<MouseInputState>,
    input: ResMut<Input<KeyCode>>,
) {
    if input.just_pressed(KeyCode::Grave) {
        mouse_input_state.use_mouse_input = !mouse_input_state.use_mouse_input;
    }
}

#[derive(Default)]
pub struct CharacterStateInputPlugin;

impl Plugin for CharacterStateInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(capture_input_state)
            .add_system(apply_input_states)
            .add_system(contact_debug::contact_debug)
            .insert_resource(InputMapping::default())
            .insert_resource(Timer::from_seconds(0.5, true))
            .insert_resource(InputStateQueue::default())
            .insert_resource(ContactDebug::default())
            .insert_resource(MouseInputState::default())
            .add_system(update_mouse_input_state)
            .register_type::<CharacterState>();
    }
}
