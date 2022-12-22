use crate::{
    csg::{self, Brush},
    render_layers,
};
use bevy::prelude::*;
use bevy_inspector_egui::Inspectable;
use serde::{Deserialize, Serialize};

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

// #[derive(Debug, Clone, Component, Serialize, Deserialize, Default)]
// pub enum EditorObject {
//     #[default]
//     None,
//     Brush(Brush),
//     PointLight(PointLightProperties),
// }

#[derive(Bundle)]
pub struct EditorObjectBrushBundle {
    pub brush: csg::Brush,
    pub csg_representation: CsgRepresentation,
    pub csg_output_link: EditorObjectOutputLink,
    pub render_layers: bevy::render::view::RenderLayers,
}

impl EditorObjectBrushBundle {
    pub fn from_brush(brush: Brush) -> Self {
        let csg: csg::Csg = brush.clone().try_into().unwrap();
        let (center, radius) = csg.bounding_sphere();

        let csg_representation = CsgRepresentation {
            center,
            radius,
            csg,
        };
        EditorObjectBrushBundle {
            brush,
            csg_representation,
            csg_output_link: default(),
            render_layers: bevy::render::view::RenderLayers::from_layers(&[
                render_layers::SIDE_2D,
                render_layers::TOP_2D,
            ]),
        }
    }
}

#[derive(Bundle)]
pub struct EditorObjectBundle {
    // pub editor_object: EditorObject,
    pub output_links: EditorObjectOutputLink,
    // pub render_layers: bevy::render::view::RenderLayers,
    pub editable_point: EditablePoint,
}

impl Default for EditorObjectBundle {
    fn default() -> Self {
        Self {
            // editor_object: Default::default(),
            output_links: Default::default(),
            // render_layers: bevy::render::view::RenderLayers::from_layers(&[
            //     render_layers::SIDE_2D,
            //     render_layers::TOP_2D,
            // ]),
            editable_point: EditablePoint,
        }
    }
}

#[derive(Component)]
pub struct EditablePoint;

#[derive(Component)]
pub struct CsgOutput;

#[derive(Component)]
pub struct SelectionVis;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Selected;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct DragAction {
    pub start_ray: Ray,
    pub action: DragActionType,
}

pub enum DragActionType {
    Face { affected_faces: Vec<(usize, f32)> },
    WholeBrush { affected_faces: Vec<(usize, f32)> },
    NonBrush { start_translation: Vec3 },
}

#[derive(Component)]
pub struct MaterialRef {
    pub material_name: String,
}

#[derive(Component)]
pub struct CsgCollisionOutput;

#[derive(Component, Inspectable)]
pub struct CsgRepresentation {
    pub center: Vec3,
    pub radius: f32,
    pub csg: csg::Csg,
}

#[derive(Component, Default)]
pub struct EditorObjectOutputLink {
    pub entities: Vec<Entity>,
}

// #[derive(Component)]
// pub struct EditorObjectLinkedBevyTransform(pub Entity);

#[derive(Component)]
pub struct Ortho2dCamera;

#[derive(Component)]
pub struct Main3dCamera;
