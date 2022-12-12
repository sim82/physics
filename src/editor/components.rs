use crate::csg::{self, Brush, Csg};
use bevy::prelude::*;
use bevy_inspector_egui::Inspectable;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub enum EditorObject {
    MinMax(Vec3, Vec3),
    Csg(Csg),
    Brush(Brush),
    PointLight,
}

#[derive(Bundle)]
pub struct BrushBundle {
    pub editor_object: EditorObject,
    pub csg_representation: CsgRepresentation,
    pub csg_output_link: CsgOutputLink,
}

impl BrushBundle {
    pub fn from_brush(brush: Brush) -> Self {
        let csg: csg::Csg = brush.clone().try_into().unwrap();
        let (center, radius) = csg.bounding_sphere();

        let csg_representation = CsgRepresentation {
            center,
            radius,
            csg,
        };
        BrushBundle {
            editor_object: EditorObject::Brush(brush),
            csg_representation,
            csg_output_link: default(),
        }
    }
}

#[derive(Component)]
pub struct CsgOutput;

#[derive(Component)]
pub struct SelectionVis;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct DragAction {
    pub start_ray: Ray,
    pub action: DragActionType,
}

pub enum DragActionType {
    Face { affected_faces: Vec<(usize, f32)> },
    WholeBrush { affected_faces: Vec<(usize, f32)> },
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
pub struct CsgOutputLink {
    pub entities: Vec<Entity>,
}
