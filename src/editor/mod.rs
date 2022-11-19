use bevy::prelude::*;

use crate::AppState;

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
        .add_startup_system(systems::setup_selection_vis_system)
        .add_startup_system(systems::setup_editor_window)
        .add_system(systems::update_brush_csg_system)
        .add_system(systems::track_primary_selection)
        .add_system(systems::track_window_props)
        .init_resource::<resources::Selection>()
        .init_resource::<resources::EditorWindows2d>();
    }
}
