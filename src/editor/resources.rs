use std::collections::BTreeMap;

use bevy::{
    prelude::*,
    utils::{hashbrown::hash_map, HashMap, HashSet},
    window::WindowId,
};
use bevy_egui::{egui, EguiContext};
use serde::{Deserialize, Serialize};

use crate::{material, sstree::SsTree};

use super::util::{self, Orientation2d};

#[derive(Default, Resource)]
pub struct Selection {
    pub primary: Option<Entity>,
    pub last_primary: Option<Entity>,
    pub last_set: Vec<Entity>,
    pub last_set_index: usize,
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
    pub offscreen_image: Handle<Image>,
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
    pub view_min: Vec3,
    pub view_max: Vec3,
}

#[derive(Resource)]
pub struct Materials {
    // pub materials: HashMap<String, Handle<StandardMaterial>>,
    pub material_defs: BTreeMap<String, material::Material>,
    pub id_to_name_map: HashMap<i32, String>, // not really the right place as this specific to the last loaded wsx file
    pub symlinks: HashMap<String, String>,
    pub dirty_symlinks: HashSet<String>,
    pub instantiated_materials: HashMap<String, Handle<StandardMaterial>>,

    // special purpose materials for 2d views
    pub brush_2d: Handle<StandardMaterial>,
    pub brush_2d_selected: Handle<StandardMaterial>,
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
            brush_2d: default(),
            brush_2d_selected: default(),
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
            hash_map::Entry::Vacant(e) => {
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

    pub fn get_brush_2d_material(&self) -> Handle<StandardMaterial> {
        self.brush_2d.clone()
    }
    pub fn get_brush_2d_selected_material(&self) -> Handle<StandardMaterial> {
        self.brush_2d_selected.clone()
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

#[derive(Default)]
pub struct WmSlot {
    pub offscreen_image: Handle<Image>,
    pub offscreen_egui_texture: egui::TextureId,
    pub target_size: egui::Vec2,
    pub current_size: wgpu::Extent3d,
    // pub drag_initial_button: util::WmMouseButton,
    pub drag_active: bool,
}

impl WmSlot {
    pub fn new(image_assets: &mut Assets<Image>, egui_context: &mut EguiContext) -> Self {
        let size = wgpu::Extent3d {
            width: 32,
            height: 32,
            ..default()
        };

        // This is the texture that will be rendered to.
        let mut image = Image {
            texture_descriptor: wgpu::TextureDescriptor {
                label: None,
                size,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                mip_level_count: 1,
                sample_count: 1,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_DST
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
            },
            ..default()
        };
        image.resize(size);
        let offscreen_image = image_assets.add(image);

        Self {
            offscreen_egui_texture: egui_context.add_image(offscreen_image.clone()),
            offscreen_image,
            target_size: egui::Vec2::new(size.width as f32, size.height as f32),
            current_size: size,
            ..default()
        }
    }

    pub fn check_resize(&mut self, image_assets: &mut Assets<Image>) {
        if self.target_size.x as u32 != self.current_size.width
            || self.target_size.y as u32 != self.current_size.height
        {
            if let Some(image) = image_assets.get_mut(&self.offscreen_image) {
                let new_size = wgpu::Extent3d {
                    width: self.target_size.x as u32,
                    height: self.target_size.y as u32,
                    ..default()
                };
                image.resize(new_size);
                self.current_size = new_size;
            }
        }
    }
}

#[derive(Default, Debug, PartialEq, Eq)]
pub enum WmSidpanelContent {
    #[default]
    Material,
    Miscsettings,
}

#[derive(Resource, Default)]
pub struct WmState {
    pub slot_upper2d: WmSlot,
    pub slot_lower2d: WmSlot,
    pub slot_main3d: WmSlot,
    pub separator_bias: f32,
    pub sidepanel_content: WmSidpanelContent,
}
