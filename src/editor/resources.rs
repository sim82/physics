use bevy::{
    prelude::*,
    utils::{HashMap, Uuid},
    window::WindowId,
};
use serde::{Deserialize, Serialize};

#[derive(Default)]
pub struct Selection {
    pub primary: Option<Entity>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
pub enum Orientation2d {
    DownFront,
    DownRight,

    Front,
    Right,
}

impl Default for Orientation2d {
    fn default() -> Self {
        Orientation2d::DownFront
    }
}

impl Orientation2d {
    pub fn flipped(&self) -> Orientation2d {
        match self {
            Orientation2d::DownFront => Orientation2d::DownRight,
            Orientation2d::DownRight => Orientation2d::DownFront,
            Orientation2d::Front => Orientation2d::Right,
            Orientation2d::Right => Orientation2d::Front,
        }
    }
    pub fn get_transform(&self) -> Transform {
        match self {
            Orientation2d::DownFront => {
                Transform::from_xyz(0.0, 6.0, 0.0).looking_at(Vec3::ZERO, Vec3::X)
            }
            Orientation2d::DownRight => {
                Transform::from_xyz(0.0, 6.0, 0.0).looking_at(Vec3::ZERO, Vec3::Z)
            }
            Orientation2d::Front => {
                Transform::from_xyz(-6.0, 0.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y)
            }
            Orientation2d::Right => {
                Transform::from_xyz(0.0, 0.0, -6.0).looking_at(Vec3::ZERO, Vec3::Y)
            }
        }
    }
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

#[derive(Default)]
pub struct EditorWindows2d {
    pub windows: HashMap<String, EditorWindow2d>,
    pub focused: Option<(String, WindowId)>,
    pub cursor_pos: Vec2,
}
