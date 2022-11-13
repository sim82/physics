use bevy::prelude::*;

#[derive(Default)]
pub struct Selection {
    pub primary: Option<Entity>,
}
