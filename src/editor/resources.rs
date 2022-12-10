use std::collections::BTreeMap;

use bevy::{
    prelude::*,
    utils::{hashbrown::hash_map, HashMap, HashSet},
    window::WindowId,
};
use bevy_egui::egui;
use serde::{Deserialize, Serialize};

use crate::{material, sstree::SsTree};

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
    pub material_defs: BTreeMap<String, material::Material>,
    pub id_to_name_map: HashMap<i32, String>, // not really the right place as this specific to the last loaded wsx file
    pub symlinks: HashMap<String, String>,
    pub dirty_symlinks: HashSet<String>,
    pub instantiated_materials: HashMap<String, Handle<StandardMaterial>>,
    // pub dirty: bool,
    // pub working_set: HashSet<Handle<StandardMaterial>>,
}

impl Default for Materials {
    fn default() -> Self {
        Self {
            material_defs: BTreeMap::new(),
            id_to_name_map: default(),
            symlinks: default(),
            // dirty: false,
            dirty_symlinks: default(),
            instantiated_materials: default(),
        }
    }
}

impl Materials {
    pub fn get(
        &mut self,
        name: &str,
        materials: &mut Assets<StandardMaterial>,
        asset_server: &mut AssetServer,
    ) -> Option<Handle<StandardMaterial>> {
        let name = if let Some(linked_name) = self.symlinks.get(name) {
            linked_name
        } else {
            name
        };
        match self.instantiated_materials.entry(name.to_string()) {
            hash_map::Entry::Occupied(e) => Some(e.get().clone()),
            hash_map::Entry::Vacant(mut e) => {
                let material = self.material_defs.get(name)?;
                Some(
                    e.insert(material::instantiate_material(
                        materials,
                        material,
                        asset_server,
                    ))
                    .clone(),
                )
            }
        }
    }

    pub fn update_symlink(&mut self, selected_appearance: String, clicked: String) {
        if let Some(linked_material) = self.symlinks.get_mut(&selected_appearance) {
            *linked_material = clicked;
            self.dirty_symlinks.insert(selected_appearance);
        }
    }
}

#[derive(Resource)]
pub struct MaterialBrowser {
    pub window_open: bool,
    pub selected_appearance: String,
    pub previews: HashMap<String, egui::TextureId>,
    // pub previews: Mutex<Previews>,
}

impl Default for MaterialBrowser {
    fn default() -> Self {
        Self {
            window_open: true,
            selected_appearance: Default::default(),
            previews: default(),
        }
    }
}

impl MaterialBrowser {
    pub fn get_preview(&self, material: &material::Material) -> Option<egui::TextureId> {
        let preview = material.preview64.as_ref().expect("missing preview");
        self.previews.get(preview).cloned()
    }

    pub fn init_previews<'a, I>(
        &mut self,
        materials: I,
        asset_server: &mut AssetServer,
        egui_context: &mut bevy_egui::EguiContext,
    ) where
        I: IntoIterator<Item = &'a material::Material>,
    {
        for material in materials.into_iter() {
            let preview = material.preview64.clone().expect("missing preview");
            match self.previews.entry(preview) {
                hash_map::Entry::Occupied(_e) => (),
                hash_map::Entry::Vacant(e) => {
                    let image: Handle<Image> = asset_server.load(e.key());
                    let texture = egui_context.add_image(image);
                    e.insert(texture);
                }
            }
        }
    }
}

#[derive(Resource, Default)]
pub struct SpatialIndex {
    pub sstree: SsTree<Entity, Vec3, 8>,
}
