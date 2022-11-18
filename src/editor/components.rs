use crate::csg::{Brush, Csg};
use bevy::prelude::*;

#[derive(Component)]
pub enum EditorObject {
    MinMax(Vec3, Vec3),
    Csg(Csg),
    Brush(Brush),
}

#[derive(Component)]
pub struct CsgOutput;
