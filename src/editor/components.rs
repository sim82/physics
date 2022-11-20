use crate::csg::{Brush, Csg};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub enum EditorObject {
    MinMax(Vec3, Vec3),
    Csg(Csg),
    Brush(Brush),
}

#[derive(Component)]
pub struct CsgOutput;

#[derive(Component)]
pub struct SelectionVis;

#[derive(Component)]
// #[component(storage = "SparseSet")]
pub struct BrushDragAction {
    pub start_ray: Ray,
    pub affected_faces: Vec<(usize, f32)>,
}
