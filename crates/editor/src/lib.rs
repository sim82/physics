use std::time::Duration;

use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;

use bevy::time::common_conditions::on_timer;
use shared::AppState;

pub mod clip_systems;
pub mod components;
pub mod edit_commands;
pub mod gui_systems;
pub mod main3d_systems;
pub mod ortho_systems;
pub mod resources;
pub mod systems;
pub mod undo;
pub mod util;
pub mod wm_systems;
pub mod wsx;

pub struct EditorPlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
#[system_set(base)]
struct FixedUpdateStage;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
#[system_set(base)]
struct TrackUpdateStage;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
#[system_set(base)]
enum CsgStage {
    Parallel,
    CommandFlush,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
#[system_set(base)]
struct PostCsgStage;

pub struct CleanupCsgOutputEvent;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(systems::setup);
        app.add_startup_system(ortho_systems::setup_editor_system)
            .init_resource::<resources::SelectionPickSet>()
            .init_resource::<resources::EditorWindows2d>();

        app.add_startup_system(clip_systems::clip_plane_setup_system.after(systems::setup)); // uses resources::materials

        app.init_resource::<undo::UndoStack>();

        app.init_resource::<resources::Materials>();
        app.init_resource::<resources::MaterialBrowser>();
        app.init_resource::<sstree::SpatialIndex>();
        app.init_resource::<resources::ClipState>();
        app.add_event::<CleanupCsgOutputEvent>();

        // system order is relatively important, since brush csg depends on some derived data to be up to date.
        // Update stage: systems that directly receive user imput and / or only manupulate 'Editor Objects'
        // app.add_system_set(
        //     SystemSet::on_update(AppState::Editor).with_system(systems::editor_input_system),
        // );
        app.add_system(systems::editor_input_system.in_set(OnUpdate(AppState::Editor)));
        // app.add_system_set(
        //     SystemSet::on_enter(AppState::Editor).with_system(ortho_systems::enter_editor_state),
        // );
        app.add_system(ortho_systems::enter_editor_state.in_schedule(OnEnter(AppState::Editor)));
        // app.add_system_set(
        //     SystemSet::on_exit(AppState::Editor).with_system(ortho_systems::leave_editor_state),
        // );
        app.add_system(ortho_systems::leave_editor_state.in_schedule(OnExit(AppState::Editor)));

        app.add_system(ortho_systems::edit_input_system)
            .add_system(ortho_systems::control_input_wm_system)
            .add_system(ortho_systems::select_input_system)
            .add_system(systems::load_save_editor_objects)
            // .add_system(ortho_systems::clip_input_system)
            .add_system(clip_systems::clip_plane_control_system);

        app.add_system(main3d_systems::select_input_system);
        app.add_system(undo::undo_system);

        // TrackUpdateStage: do 'first order' post processing based on user interaction, e.g.:
        //  - update spatial index of Editor Objects
        //  - track 2d vis meshes (note: ordered after track_brush_updates, since the all-entity origin is updated there)
        //  - derive Editor Object origin from brush geometry
        //  - despawn

        // app.add_stage_after(
        //     CoreStage::Update,
        //     TrackUpdateStage,
        //     SystemStage::parallel()
        //         .with_system(systems::update_symlinked_materials_system)
        //         .with_system(ortho_systems::adjust_clip_planes_system)
        //         .with_system(systems::track_lights_system)
        //         .with_system(systems::track_brush_updates)
        //         .with_system(clip_systems::clip_plane_vis_system)
        //         .with_system(systems::track_2d_vis_system.after(systems::track_brush_updates)),
        // );
        app.configure_set(
            TrackUpdateStage
                .after(CoreSet::UpdateFlush)
                .before(CoreSet::PostUpdate),
        );
        app.add_systems(
            (
                systems::update_symlinked_materials_system,
                ortho_systems::adjust_clip_planes_system,
                systems::track_lights_system,
                systems::track_brush_updates,
                clip_systems::clip_plane_vis_system,
                systems::track_2d_vis_system.after(systems::track_brush_updates),
            )
                .in_base_set(TrackUpdateStage),
        );

        // CsgStage: incremental CSG update. Deferred update with fixed timestep. Needs all 'fist order' post
        // procesing data to be up to date (especially derived translation for brushed, since output mesh is
        // attached to Editor Object and needs the correct origin. Also the spatial index should better be up to date...)
        // app.add_stage_after(
        //     TrackUpdateStage,
        //     CsgStage,
        //     SystemStage::parallel()
        //         .with_run_criteria(FixedTimestep::step(0.1))
        //         .with_system(wm_systems::write_view_settings) // running as guest, just for fixed timestep...
        //         .with_system(systems::create_brush_csg_system_inc),
        // )l
        // app.configure_sets(CsgStage.after(TrackUpdateStage));
        app.configure_sets((CsgStage::Parallel, CsgStage::CommandFlush).chain())
            .add_system(apply_system_buffers.in_base_set(CsgStage::CommandFlush));
        app.add_system(
            systems::create_brush_csg_system_inc
                // .run_if(on_timer(Duration::from_millis(100)))
                .in_base_set(CsgStage::Parallel),
        );
        // add.
        // PostCsgStage: update stuff that depends on Csg output, e.g. resolve material refs to bevy material
        // app.add_stage_after(
        //     CsgStage,
        //     PostCsgStage,
        //     SystemStage::parallel()
        //         .with_system(systems::update_material_refs_system)
        //         .with_system(systems::track_primary_selection) // must run after track_2d_vis_system
        //         .with_system(clip_systems::clip_preview_system), // .with_system(ortho_systems::clip_point_update_system)
        // );
        app.configure_set(PostCsgStage.after(CsgStage::CommandFlush));
        app.add_systems(
            (
                systems::update_material_refs_system,
                systems::track_primary_selection, // must run after track_2d_vis_system
                clip_systems::clip_preview_system, // .with_system(ortho_systems::clip_point_update_system)
            )
                .in_base_set(PostCsgStage),
        );
        // app.register_inspectable::<components::CsgRepresentation>();
        // app.register_inspectable::<components::BrushMaterialProperties>();

        // Wm test
        app.init_resource::<resources::WmState>();
        app.add_startup_system(
            wm_systems::wm_test_setup_system.in_base_set(StartupSet::PreStartup),
        );

        // app.add_system_set(
        //     SystemSet::on_update(AppState::Editor).with_system(wm_systems::wm_test_system),
        // );
        app.add_system(wm_systems::wm_test_system.in_set(OnUpdate(AppState::Editor)));
        app.add_event::<util::WmEvent>();
        app.add_system(
            wm_systems::write_view_settings.run_if(on_timer(Duration::from_millis(500))),
        );
    }
}

pub struct EditorPluginGroup;
impl PluginGroup for EditorPluginGroup {
    fn build(self) -> bevy::app::PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(EditorPlugin)
            // .add(debug_gui::DebugGuiPlugin)
            .add(bevy_infinite_grid::InfiniteGridPlugin)
    }
}
