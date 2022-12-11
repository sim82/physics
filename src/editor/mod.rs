use bevy::{prelude::*, time::FixedTimestep};

use crate::AppState;

pub mod components;
pub mod gui_systems;
pub mod ortho_systems;
pub mod resources;
pub mod systems;
pub mod util;

pub struct EditorPlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
struct FixedUpdateStage;

pub struct CleanupCsgOutputEvent;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(systems::setup);
        app.init_resource::<resources::Materials>();
        app.init_resource::<resources::MaterialBrowser>();
        app.init_resource::<resources::SpatialIndex>();
        app.add_event::<CleanupCsgOutputEvent>();
        app.add_system_set(
            SystemSet::on_update(AppState::DebugMenu).with_system(systems::editor_input_system),
        )
        .add_startup_system(systems::setup_selection_vis_system.after(systems::setup))
        // .add_system(systems::cleanup_brush_csg_system.after(systems::update_material_refs))
        // .add_system(systems::create_brush_csg_system.after(systems::cleanup_brush_csg_system))
        // .add_system(systems::update_material_refs)
        .add_system(systems::track_primary_selection)
        .add_startup_system(ortho_systems::setup_editor_window)
        .init_resource::<resources::Selection>()
        .init_resource::<resources::EditorWindows2d>();

        app.add_system(ortho_systems::track_window_props)
            .add_system(ortho_systems::track_focused_window)
            .add_system(ortho_systems::edit_input_system)
            .add_system(ortho_systems::control_input_system)
            .add_system(ortho_systems::select_input_system)
            .add_system(systems::load_save_editor_objects);

        app.add_system(gui_systems::materials_egui_system);

        // fixed timestep stage for non realtime stuff like writing config
        app.add_stage_after(
            CoreStage::Update,
            FixedUpdateStage,
            SystemStage::parallel()
                .with_run_criteria(FixedTimestep::step(0.5))
                .with_system(systems::cleanup_brush_csg_system)
                .with_system(systems::create_brush_csg_system)
                .with_system(systems::update_material_refs)
                .with_system(ortho_systems::write_window_settings),
        );

        app.add_system_to_stage(CoreStage::PostUpdate, systems::update_material_refs);
        app.add_system_to_stage(CoreStage::PostUpdate, systems::update_symlinked_materials);
        app.add_system_to_stage(CoreStage::PostUpdate, systems::track_2d_vis_system);
        app.add_system_to_stage(CoreStage::PostUpdate, ortho_systems::adjust_clip_planes);
    }
}
