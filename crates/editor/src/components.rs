use std::f32::consts::PI;

use bevy::prelude::*;
use csg::{self, Brush};
use serde::{Deserialize, Serialize};

use sstree::SpatialBounds;

#[derive(Debug, Clone, Serialize, Deserialize, Component)]
pub struct PointLightProperties {
    pub shadows_enabled: bool,
    pub range: f32,
}

impl Default for PointLightProperties {
    fn default() -> Self {
        Self {
            shadows_enabled: false,
            range: 5.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Component)]
pub struct DirectionalLightProperties {
    pub shadows_enabled: bool,
    pub half_size: f32,
}

impl Default for DirectionalLightProperties {
    fn default() -> Self {
        Self {
            shadows_enabled: false,
            half_size: 10.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Component, Reflect, Default)]
// #[reflect(Component)]
pub struct BrushMaterialProperties {
    pub materials: Vec<String>,
}

// #[derive(Debug, Clone, Component, Serialize, Deserialize, Default)]
// pub enum EditorObject {
//     #[default]
//     None,
//     Brush(Brush),
//     PointLight(PointLightProperties),
// }

#[derive(Bundle)]
pub struct EditorObjectBrushBundle {
    pub spatial_bundle: SpatialBundle,
    pub brush: csg::Brush,
    pub csg_representation: CsgRepresentation,
    pub material_properties: BrushMaterialProperties,
    // pub csg_output_link: EditorObjectOutputLink,
    // pub render_layers: bevy::render::view::RenderLayers,
    pub name: Name,
    pub csg_dirty: CsgDirty,
}

impl EditorObjectBrushBundle {
    pub fn from_brush(brush: Brush) -> Self {
        let csg: csg::Csg = brush.clone().try_into().unwrap();
        let (center, radius) = csg.bounding_sphere();

        let csg_representation = CsgRepresentation {
            bounds: SpatialBounds { center, radius },
            csg,
        };
        EditorObjectBrushBundle {
            spatial_bundle: default(),
            material_properties: BrushMaterialProperties {
                materials: std::iter::repeat(String::from("material/architecture/woodframe1"))
                    .take(brush.appearances.len())
                    .collect(),
            },
            brush,
            csg_representation,
            // csg_output_link: default(),
            // render_layers: bevy::render::view::RenderLayers::from_layers(&[
            //     render_layers::SIDE_2D,
            //     render_layers::TOP_2D,
            // ]),
            name: Name::new("Brush"),
            csg_dirty: CsgDirty,
        }
    }

    pub fn with_material_properties(
        mut self,
        material_properties: BrushMaterialProperties,
    ) -> Self {
        self.material_properties = material_properties;
        self
    }
}

#[derive(Bundle)]
pub struct EditorObjectPointlightBundle {
    // pub editor_object: EditorObject,
    // pub output_links: EditorObjectOutputLink,
    // pub render_layers: bevy::render::view::RenderLayers,
    pub spatial: SpatialBundle,
    pub editable_point: EditablePoint,
    pub light_properties: PointLightProperties,
    pub name: Name,
}

impl Default for EditorObjectPointlightBundle {
    fn default() -> Self {
        Self {
            // editor_object: Default::default(),
            // output_links: Default::default(),
            // render_layers: bevy::render::view::RenderLayers::from_layers(&[
            //     render_layers::SIDE_2D,
            //     render_layers::TOP_2D,
            // ]),
            spatial: SpatialBundle::default(),
            light_properties: PointLightProperties {
                shadows_enabled: true,
                ..default()
            },
            editable_point: EditablePoint,
            name: Name::new("PointLight"),
        }
    }
}

#[derive(Bundle)]
pub struct EditorObjectDirectionalLightBundle {
    // pub editor_object: EditorObject,
    // pub output_links: EditorObjectOutputLink,
    // pub render_layers: bevy::render::view::RenderLayers,
    pub spatial: SpatialBundle,
    pub editable_point: EditablePoint,
    pub light_properties: DirectionalLightProperties,
    pub name: Name,
}

impl Default for EditorObjectDirectionalLightBundle {
    fn default() -> Self {
        // directional 'sun' light

        Self {
            // editor_object: Default::default(),
            // output_links: Default::default(),
            // render_layers: bevy::render::view::RenderLayers::from_layers(&[
            //     render_layers::SIDE_2D,
            //     render_layers::TOP_2D,
            // ]),
            spatial: SpatialBundle::from_transform(Transform::from_rotation(
                Quat::from_rotation_x(-PI / 4.),
            )),
            light_properties: DirectionalLightProperties {
                shadows_enabled: true,
                ..default()
            },
            editable_point: EditablePoint,
            name: Name::new("DirectionalLight"),
        }
    }
}

#[derive(Component)]
pub struct EditablePoint;

#[derive(Component)]
pub struct CsgOutput;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Selected;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct DragAction {
    pub start_ray: Ray3d,
    pub action: DragActionType,
}

pub enum DragActionType {
    Face { affected_faces: Vec<(usize, f32)> },
    WholeBrush { affected_faces: Vec<(usize, f32)> },
    NonBrush { start_translation: Vec3 },
}

#[derive(Component)]
#[component(storage = "SparseSet")]
pub enum EditUpdate {
    BrushDrag {
        brush: csg::Brush,
        // csg_reprensentation: CsgRepresentation,
    },
}

#[derive(Component)]
pub struct MaterialRef {
    pub material_name: String,
}

#[derive(Component)]
pub struct CsgCollisionOutput;

#[derive(Component, Reflect, Clone)]
pub struct CsgRepresentation {
    pub bounds: SpatialBounds,
    pub csg: csg::Csg,
}

#[derive(Component)]
pub struct ProcessedCsg {
    pub bsp: csg::Node,
}

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct CsgDirty;

// #[derive(Component, Default)]
// pub struct EditorObjectOutputLink {
//     pub entities: Vec<Entity>,
// }

// #[derive(Component)]
// pub struct EditorObjectLinkedBevyTransform(pub Entity);

#[derive(Component)]
pub struct Ortho2dCamera;

#[derive(Component)]
pub struct Main3dCamera;

#[derive(Component)]
pub struct SelectionHighlighByMaterial;

#[derive(Component)]
pub enum SelectionHighlightByGizmo {
    Sphere { radius: f32 },
}

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Despawn;

#[derive(Component)]
pub struct ClipPreview;

#[derive(Component)]
pub struct ClipPoint0;

#[derive(Component)]
pub struct ClipPoint1;

#[derive(Component)]
pub struct ClipPoint2;

#[derive(Component)]
pub struct ClipPlane {
    pub points: [Vec3; 3],
}

impl Default for ClipPlane {
    fn default() -> Self {
        Self {
            points: [
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(1.0, 1.0, 0.0),
            ],
        }
    }
}

impl ClipPlane {
    // pub fn update_plane(&mut self) {
    //     self.plane = csg::Plane::from_points_slice(&self.input_points);
    // }

    pub fn get_plane(&self) -> csg::Plane {
        csg::Plane::from_points_slice(&self.points)
    }
}

#[derive(Bundle)]
pub struct ClipPlaneBundle {
    pub clip_plane: ClipPlane,
    pub spatial_bundle: SpatialBundle,
    pub name: Name,
}

impl Default for ClipPlaneBundle {
    fn default() -> Self {
        Self {
            clip_plane: Default::default(),
            spatial_bundle: Default::default(),
            name: Name::new("clip_plane"),
        }
    }
}
