use bevy::prelude::*;

use crate::AppState;

use self::{
    resources::Selection,
    systems::{
        setup_editor_window, setup_selection_vis_system, track_primary_selection,
        update_brush_csg_system,
    },
};

pub mod components;
pub mod resources;
pub mod systems;
pub mod util;

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(
            SystemSet::on_update(AppState::DebugMenu).with_system(systems::editor_input_system),
        )
        // .add_system(update_brushes_system)
        .add_startup_system(setup_selection_vis_system)
        .add_startup_system(setup_editor_window)
        .add_system(update_brush_csg_system)
        .add_system(track_primary_selection)
        .init_resource::<Selection>();
    }
}
