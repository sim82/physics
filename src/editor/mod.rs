use bevy::{prelude::*, time::FixedTimestep};
use bevy_inspector_egui::RegisterInspectable;

use crate::AppState;

pub mod components;
pub mod gui_systems;
pub mod main3d_systems;
pub mod ortho_systems;
pub mod resources;
pub mod systems;
pub mod undo;
pub mod util;
pub mod wm_systems;

pub struct EditorPlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
struct FixedUpdateStage;

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
struct TrackUpdateStage;

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
struct CsgStage;

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
struct PostCsgStage;

pub struct CleanupCsgOutputEvent;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(systems::setup);
        app.add_startup_system(ortho_systems::setup_editor_system)
            .init_resource::<resources::Selection>()
            .init_resource::<resources::EditorWindows2d>();

        app.init_resource::<undo::UndoStack>();

        app.init_resource::<resources::Materials>();
        app.init_resource::<resources::MaterialBrowser>();
        app.init_resource::<resources::SpatialIndex>();
        app.add_event::<CleanupCsgOutputEvent>();

        // system order is relatively important, since brush csg depends on some derived data to be up to date.
        // Update stage: systems that directly receive user imput and / or only manupulate 'Editor Objects'
        app.add_system_set(
            SystemSet::on_update(AppState::Editor).with_system(systems::editor_input_system),
        );
        app.add_system_set(
            SystemSet::on_enter(AppState::Editor).with_system(ortho_systems::enter_editor_state),
        );
        app.add_system_set(
            SystemSet::on_exit(AppState::Editor).with_system(ortho_systems::leave_editor_state),
        );

        app.add_system(ortho_systems::edit_input_system)
            .add_system(ortho_systems::control_input_wm_system)
            .add_system(ortho_systems::select_input_system)
            .add_system(systems::load_save_editor_objects);

        app.add_system(main3d_systems::select_input_system);
        app.add_system(undo::undo_system);

        // TrackUpdateStage: do 'first order' post processing based on user interaction, e.g.:
        //  - update spacial index of Editor Objects
        //  - track 2d vis meshes
        //  - derive Editor Object origin from brush geometry

        app.add_stage_after(
            CoreStage::Update,
            TrackUpdateStage,
            SystemStage::parallel()
                .with_system(systems::update_symlinked_materials_system)
                .with_system(systems::track_2d_vis_system)
                .with_system(ortho_systems::adjust_clip_planes_system)
                .with_system(systems::track_lights_system)
                .with_system(systems::track_brush_updates),
        );

        // CsgStage: incremental CSG update. Deferred update with fixed timestep. Needs all 'fist order' post
        // procesing data to be up to date (especially derived translation for brushed, since output mesh is
        // attached to Editor Object and needs the correct origin. Also the spatial index should better be up to date...)
        app.add_stage_after(
            TrackUpdateStage,
            CsgStage,
            SystemStage::parallel()
                .with_run_criteria(FixedTimestep::step(0.1))
                .with_system(ortho_systems::write_window_settings) // running as guest, just for fixed timestep...
                .with_system(systems::create_brush_csg_system_inc),
        );

        // PostCsgStage: update stuff that depends on Csg output, e.g. resolve material refs to bevy material
        app.add_stage_after(
            CsgStage,
            PostCsgStage,
            SystemStage::parallel()
                .with_system(systems::update_material_refs_system)
                .with_system(systems::track_primary_selection), // must run after track_2d_vis_system
        );

        app.register_inspectable::<components::CsgRepresentation>();
        app.register_inspectable::<components::BrushMaterialProperties>();

        // Wm test
        app.init_resource::<resources::WmState>();
        app.add_startup_system_to_stage(StartupStage::PreStartup, wm_systems::wm_test_setup_system);

        app.add_system_set(
            SystemSet::on_update(AppState::Editor).with_system(wm_systems::wm_test_system),
        );
        app.add_event::<util::WmEvent>();
    }
}
