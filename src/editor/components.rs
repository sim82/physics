use crate::csg::Csg;
use bevy::prelude::*;

#[derive(Component)]
pub enum Brush {
    MinMax(Vec3, Vec3),
    Csg(Csg),
}
