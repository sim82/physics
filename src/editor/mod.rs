use bevy::{prelude::*, time::FixedTimestep};
use bevy_inspector_egui::RegisterInspectable;

use crate::AppState;

pub mod components;
pub mod gui_systems;
pub mod ortho_systems;
pub mod resources;
pub mod systems;
pub mod util;
pub mod wm_systems;

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
            SystemSet::on_update(AppState::Editor).with_system(systems::editor_input_system),
        );
        app.add_system_set(
            SystemSet::on_enter(AppState::Editor).with_system(ortho_systems::enter_editor_state),
        );
        app.add_system_set(
            SystemSet::on_exit(AppState::Editor).with_system(ortho_systems::leave_editor_state),
        );

        app.add_startup_system(systems::setup_selection_vis_system.after(systems::setup))
            .add_system(systems::track_primary_selection)
            .add_startup_system(ortho_systems::setup_editor_system)
            .init_resource::<resources::Selection>()
            .init_resource::<resources::EditorWindows2d>();

        app
            // .add_system(ortho_systems::track_window_props)
            // .add_system(ortho_systems::track_focused_window)
            .add_system(ortho_systems::edit_input_system)
            .add_system(ortho_systems::control_input_wm_system)
            .add_system(ortho_systems::select_input_system)
            .add_system(systems::load_save_editor_objects);

        // fixed timestep stage for non realtime stuff like writing config
        app.add_stage_after(
            CoreStage::Update,
            FixedUpdateStage,
            SystemStage::parallel()
                .with_run_criteria(FixedTimestep::step(0.1))
                .with_system(
                    systems::create_brush_csg_system_inc.after(systems::track_spatial_index_system),
                )
                .with_system(systems::update_material_refs_system)
                .with_system(ortho_systems::write_window_settings),
        );

        app.add_system_to_stage(CoreStage::PostUpdate, systems::update_material_refs_system);
        app.add_system_to_stage(
            CoreStage::PostUpdate,
            systems::update_symlinked_materials_system,
        );
        app.add_system_to_stage(CoreStage::PostUpdate, systems::track_2d_vis_system);
        app.add_system_to_stage(
            CoreStage::PostUpdate,
            ortho_systems::adjust_clip_planes_system,
        );
        app.add_system_to_stage(CoreStage::PostUpdate, systems::track_lights_system);
        app.add_system_to_stage(CoreStage::PostUpdate, systems::track_spatial_index_system);
        app.add_system_to_stage(
            CoreStage::PostUpdate,
            systems::track_linked_transforms_system,
        );

        app.register_inspectable::<components::CsgRepresentation>();

        // Wm test
        app.init_resource::<resources::WmState>();
        app.add_startup_system_to_stage(StartupStage::PreStartup, wm_systems::wm_test_setup_system);

        app.add_system_set(
            SystemSet::on_update(AppState::Editor).with_system(wm_systems::wm_test_system),
        );
        app.add_event::<util::WmEvent>();
    }
}
