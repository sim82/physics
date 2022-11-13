use bevy::{app::AppExit, prelude::*};

pub mod contact_debug;
// pub mod debug_lines;
pub mod editor;
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

pub mod player_controller;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum AppState {
    DebugMenu,
    InGame,
    // Paused,
}

mod attic;
pub fn exit_on_esc_system(
    keyboard_input: Res<Input<KeyCode>>,
    mut app_exit_events: EventWriter<AppExit>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        app_exit_events.send_default();
    }
}
