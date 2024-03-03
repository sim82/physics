use bevy::prelude::*;

// TODO: port
pub fn adjust_grid_system(// keycodes: Res<ButtonInput<KeyCode>>,

    // mut editor_windows_2d: ResMut<resources::EditorWindows2d>,
    // mut camera_query: Query<(&GlobalTransform, &Camera, &mut Projection, &mut Transform)>,
    // mut grid_query: Query<(&mut Transform, &mut bevy_infinite_grid::InfiniteGrid), Without<Camera>>,
) {
    //     let editor_windows_2d = &mut *editor_windows_2d;

    //     let Some(upper) = editor_windows_2d.windows.get(UPPER_WINDOW) else {
    //         return;
    //     };
    //     let Some(lower) = editor_windows_2d.windows.get(LOWER_WINDOW) else {
    //         return;
    //     };

    //     let upper_orientation = &upper.orientation;
    //     let lower_orientation = &lower.orientation;

    //     let Ok((upper_transform, upper_camera, upper_projection, _)) = camera_query.get(upper.camera)
    //     else {
    //         return;
    //     };

    //     let Ok((lower_transform, lower_camera, _lower_projection, _)) = camera_query.get(lower.camera)
    //     else {
    //         return;
    //     };

    //     let scaling = if let Projection::Orthographic(OrthographicProjection {
    //         scaling_mode: ScalingMode::FixedHorizontal(scaling),
    //         ..
    //     }) = upper_projection
    //     {
    //         *scaling
    //     } else {
    //         warn!("failed to get scaling factor from ortographic projection");
    //         1.0
    //     };

    //     // meh I guess there is a formula for that...
    //     let mut trunc_scaling = 1.0;
    //     while trunc_scaling * 2.0 < scaling {
    //         trunc_scaling *= 10.0
    //     }
    //     // info!("scaling: {} -> {}", scaling, trunc_scaling);

    //     let grid_scaling = 100.0 / trunc_scaling;

    //     let Some((upper_min, upper_max)) = ortho_view_bounds(upper_camera, upper_transform) else {
    //         return;
    //     };
    //     let Some((lower_min, lower_max)) = ortho_view_bounds(lower_camera, lower_transform) else {
    //         return;
    //     };

    //     {
    //         let min = upper_orientation.get_up_axis(upper_min) - 5.0;
    //         let max = upper_orientation.get_up_axis(upper_max) + 5.0;

    //         let Ok((_, _, mut lower_projection, mut lower_transform)) =
    //             camera_query.get_mut(lower.camera)
    //         else {
    //             return;
    //         };
    //         let Projection::Orthographic(lower_ortho) = &mut *lower_projection else {
    //             return;
    //         };

    //         let Ok((mut lower_grid_transform, _)) = grid_query.get_mut(lower.grid) else {
    //             return;
    //         };

    //         *upper_orientation.get_up_axis_mut(&mut lower_transform.translation) = max;
    //         *upper_orientation.get_up_axis_mut(&mut lower_grid_transform.translation) = min + 0.1;
    //         lower_grid_transform.scale.x = grid_scaling;
    //         lower_ortho.far = max - min;
    //         // info!("depth: {}", lower_ortho.far);
    //         *upper_orientation.get_up_axis_mut(&mut editor_windows_2d.view_max) = max;
    //         *upper_orientation.get_up_axis_mut(&mut editor_windows_2d.view_min) = min;
    //     }

    //     {
    //         let min = lower_orientation.get_up_axis(lower_min) - 5.0;
    //         let max = lower_orientation.get_up_axis(lower_max) + 5.0;

    //         let Ok((_, _, mut upper_projection, mut upper_transform)) =
    //             camera_query.get_mut(upper.camera)
    //         else {
    //             return;
    //         };
    //         let Projection::Orthographic(upper_ortho) = &mut *upper_projection else {
    //             return;
    //         };
    //         let Ok((mut upper_grid_transform, _)) = grid_query.get_mut(upper.grid) else {
    //             return;
    //         };

    //         *lower_orientation.get_up_axis_mut(&mut upper_transform.translation) = max;
    //         *lower_orientation.get_up_axis_mut(&mut upper_grid_transform.translation) = min + 0.1;
    //         upper_grid_transform.scale.x = grid_scaling;

    //         upper_ortho.far = max - min;
    //         *lower_orientation.get_up_axis_mut(&mut editor_windows_2d.view_max) = max;
    //         *lower_orientation.get_up_axis_mut(&mut editor_windows_2d.view_min) = min;
    //     }

    //     if keycodes.just_pressed(KeyCode::F2) {
    //         let mut right = 0.0;
    //         if let Some(window) = editor_windows_2d.windows.get_mut(resources::UPPER_WINDOW) {
    //             window.orientation = window.orientation.flipped();
    //             if let Ok((_, _, _, mut transform)) = camera_query.get_mut(window.camera) {
    //                 transform.rotation = window.orientation.get_transform().rotation;
    //                 right = window.orientation.get_right_axis(transform.translation);
    //             };
    //         }
    //         if let Some(window) = editor_windows_2d.windows.get_mut(resources::LOWER_WINDOW) {
    //             window.orientation = window.orientation.flipped();
    //             if let Ok((_, _, _, mut transform)) = camera_query.get_mut(window.camera) {
    //                 transform.rotation = window.orientation.get_transform().rotation;
    //                 *window
    //                     .orientation
    //                     .get_right_axis_mut(&mut transform.translation) = right;
    //             };
    //             if let Ok((mut lower_grid_transform, mut grid)) = grid_query.get_mut(window.grid) {
    //                 // lower_grid_transform
    //                 *lower_grid_transform =
    //                     Transform::from_rotation(window.orientation.get_grid_rotation());
    //                 grid.x_axis_color = window.orientation.get_lower_x_axis_color();
    //                 grid.z_axis_color = window.orientation.get_lower_z_axis_color();
    //             };
    //         }
    //     }
}
#[derive(Bundle)]
pub struct GridBundle {}
// init code:

impl GridBundle {
    pub fn new(
        lower_window: bool,
        t: crate::util::Orientation2d,
        render_layer: bevy::render::view::RenderLayers,
    ) -> Self {
        // todo!()
        // #[cfg(feature = "external_deps")]
        // let grid_entity = {
        //     let grid = if name == LOWER_WINDOW {
        //         bevy_infinite_grid::InfiniteGrid {
        //             x_axis_color: t.get_lower_x_axis_color(),
        //             z_axis_color: t.get_lower_z_axis_color(),
        //             ..Default::default()
        //         }
        //     } else {
        //         // upper grid uses default colors
        //         default()
        //     };

        //     let grid_entity = commands
        //         .spawn((
        //             bevy_infinite_grid::InfiniteGridBundle {
        //                 grid,
        //                 transform: Transform::from_rotation(t.get_grid_rotation()),
        //                 ..Default::default()
        //             },
        //             render_layer,
        //             Name::new(format!("{} grid", name)),
        //         ))
        //         .id();
        // };
        GridBundle {}
    }
}

pub struct GridPlugin;

impl Plugin for GridPlugin {
    fn build(&self, app: &mut App) {
        // app.add_plugins(bevy_infinite_grid::InfiniteGridPlugin);
    }
}
