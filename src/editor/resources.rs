use bevy::{prelude::*, utils::HashMap, window::WindowId};
use serde::{Deserialize, Serialize};

use super::util::Orientation2d;

#[derive(Default, Resource)]
pub struct Selection {
    pub primary: Option<Entity>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Default)]
pub struct EditorWindowSettings {
    pub pos_x: i32,
    pub pos_y: i32,
    pub width: i32,
    pub height: i32,
    pub orientation: Orientation2d,
}

pub struct EditorWindow2d {
    pub camera: Entity,
    pub window_id: WindowId,
    pub settings: EditorWindowSettings,
}

pub const UPPER_WINDOW: &str = "upper";
pub const LOWER_WINDOW: &str = "lower";

#[derive(Default, Resource)]
pub struct EditorWindows2d {
    pub windows: HashMap<String, EditorWindow2d>,
    pub focused: Option<(String, WindowId)>,
    pub cursor_pos: Vec2,
}
