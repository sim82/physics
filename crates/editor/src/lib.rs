use std::time::Duration;

use bevy::app::PluginGroupBuilder;
use bevy::ecs::schedule::ScheduleLabel;
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

// #[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
// // #[system_set(base)]
// enum EditorSet {
//     TrackUpdate,
//     TrackUpdateFlush,
//     Csg,
//     CsgFlush,
//     PostCsg,
// }

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, SystemSet)]
struct EditorTrackUpdate;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, SystemSet)]
struct EditorTrackUpdateFlush;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, SystemSet)]
struct EditorCsg;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, SystemSet)]
struct EditorCsgFlush;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, SystemSet)]
struct EditorPostCsg;

// #[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Debug)]
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, SystemSet)]
// #[system_set(base)]
struct PostCsgStage;

#[derive(Event)]
pub struct CleanupCsgOutputEvent;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            (
                systems::setup,
                ortho_systems::setup_editor_system,
                clip_systems::clip_plane_setup_system.after(systems::setup),
            ),
        ); // uses resources::materials

        app.init_resource::<undo::UndoStack>();
        app.init_resource::<resources::SelectionPickSet>();
        app.init_resource::<resources::EditorWindows2d>();
        app.init_resource::<resources::Materials>();
        app.init_resource::<resources::MaterialBrowser>();
        app.init_resource::<sstree::SpatialIndex>();
        app.init_resource::<resources::ClipState>();
        app.add_event::<CleanupCsgOutputEvent>();

        app.add_systems(
            Update,
            systems::editor_input_system.run_if(in_state(AppState::Editor)),
        );
        app.add_systems(OnEnter(AppState::Editor), ortho_systems::enter_editor_state);
        app.add_systems(OnExit(AppState::Editor), ortho_systems::leave_editor_state);

        // system order is relatively important, since brush csg depends on some derived data to be up to date.
        // editing of csg brushes involves four stages that need command flushes in between them to prevent flickering:

        // (CoreSet) Update stage: systems that directly receive user input and / or only manipulate 'Editor Objects'

        // TrackUpdateStage: do 'first order' post processing based on user interaction, e.g.:
        //  - update spatial index of Editor Objects
        //  - track 2d vis meshes (note: ordered after track_brush_updates, since the all-entity origin is updated there)
        //  - derive Editor Object origin from brush geometry
        //  - despawn

        // CsgStage: incremental CSG update. Deferred update with fixed timestep. Needs all 'fist order' post
        // processing data to be up to date (especially derived translation for brushes, since output mesh is
        // attached to Editor Object and needs the correct origin. Also the spatial index should better be up to date...)

        // PostCsgStage:
        //  - update material refs (especially set material for meshes created in CsgStage, otherwise the pink default material will be visible)
        //  - update clip preview and primary selection vis
        app.configure_sets(
            Update,
            (
                EditorTrackUpdate,
                EditorTrackUpdateFlush,
                EditorCsg,
                EditorCsgFlush,
                EditorPostCsg,
            )
                .chain(),
        )
        .add_systems(Update, apply_deferred.in_set(EditorTrackUpdateFlush))
        .add_systems(Update, apply_deferred.in_set(EditorCsgFlush));
        app.add_systems(
            Update,
            (
                ortho_systems::edit_input_system,
                ortho_systems::control_input_wm_system,
                ortho_systems::select_input_system,
                systems::load_save_editor_objects,
                clip_systems::clip_plane_control_system,
                main3d_systems::select_input_system,
            ),
        );

        app.add_systems(
            EditorTrackUpdate,
            (
                systems::update_symlinked_materials_system,
                ortho_systems::adjust_clip_planes_system,
                systems::track_lights_system,
                systems::track_brush_updates,
                clip_systems::clip_plane_vis_system,
                systems::track_2d_vis_system.after(systems::track_brush_updates),
                systems::track_wireframe_system.after(systems::track_brush_updates),
            ),
        );
        app.add_systems(
            EditorCsg,
            systems::create_brush_csg_system_inc.run_if(on_timer(Duration::from_millis(100))), // .run_if(on_timer(Duration::from_millis(1000 / 15)))
        );
        app.add_systems(
            EditorPostCsg,
            (
                systems::update_material_refs_system,
                systems::track_primary_selection, // must run after track_2d_vis_system
                clip_systems::clip_preview_system, // .with_system(ortho_systems::clip_point_update_system)
            ),
        );
        app.register_type::<components::CsgRepresentation>();
        app.register_type::<components::BrushMaterialProperties>();

        // Wm test
        app.init_resource::<resources::WmState>();
        app.add_systems(PreStartup, wm_systems::wm_test_setup_system);

        // app.add_system_set(
        //     SystemSet::on_update(AppState::Editor).with_system(wm_systems::wm_test_system),
        // );
        app.add_systems(
            Update,
            wm_systems::wm_test_system.run_if(in_state(AppState::Editor)),
        );
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
