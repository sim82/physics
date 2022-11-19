use bevy::{prelude::*, time::FixedTimestep};

use crate::AppState;

pub mod components;
pub mod resources;
pub mod systems;
pub mod util;

pub struct EditorPlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
struct FixedUpdateStage;

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
        .add_system(systems::track_focused_window)
        .add_system(systems::editor_windows_2d_input_system)
        .init_resource::<resources::Selection>()
        .init_resource::<resources::EditorWindows2d>();

        // fixed timestep stage for non realtime stuff like writing config
        app.add_stage_after(
            CoreStage::Update,
            FixedUpdateStage,
            SystemStage::parallel()
                .with_run_criteria(FixedTimestep::step(0.1))
                // .with_system(systems::update_brush_csg_system),
                .with_system(systems::write_window_settings),
        );
    }
}
