use bevy::{prelude::*, utils::HashMap, window::WindowId};
use serde::{Deserialize, Serialize};

use crate::material;

use super::util::Orientation2d;

#[derive(Default, Resource)]
pub struct Selection {
    pub primary: Option<Entity>,
    pub last_primary: Option<Entity>,
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

pub struct TranslateDrag {
    pub start_ray: Ray,
    pub start_focus: String,
    pub start_global_transform: GlobalTransform,
    pub start_transforms: Vec<(Entity, Transform)>,
}

pub const UPPER_WINDOW: &str = "upper";
pub const LOWER_WINDOW: &str = "lower";

#[derive(Default, Resource)]
pub struct EditorWindows2d {
    pub windows: HashMap<String, EditorWindow2d>,
    pub focused: Option<(String, WindowId)>,
    pub cursor_pos: Vec2,
    pub translate_drag: Option<TranslateDrag>,
}

#[derive(Resource)]
pub struct Materials {
    // pub materials: HashMap<String, Handle<StandardMaterial>>,
    pub material_defs: HashMap<String, material::Material>,
    pub id_to_name_map: HashMap<i32, String>, // not really the right place as this specific to the last loaded wsx file
    pub symlinks: HashMap<String, String>,
}

impl Default for Materials {
    fn default() -> Self {
        Self {
            material_defs: HashMap::new(),
            id_to_name_map: default(),
            symlinks: default(),
        }
    }
}

impl Materials {
    pub fn get(
        &self,
        name: &str,
        materials: &mut Assets<StandardMaterial>,
        asset_server: &mut AssetServer,
    ) -> Option<Handle<StandardMaterial>> {
        let name = if let Some(linked_name) = self.symlinks.get(name) {
            linked_name
        } else {
            name
        };
        let material = self.material_defs.get(name)?;
        Some(material::instantiate_material(
            materials,
            material,
            asset_server,
        ))
    }
}
