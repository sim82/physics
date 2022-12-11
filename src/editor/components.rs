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
