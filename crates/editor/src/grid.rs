use bevy::prelude::*;
use shared::render_layers;

use crate::{
    resources::{self, LOWER_WINDOW, UPPER_WINDOW},
    util::{ortho_view_bounds, Orientation2d},
};

#[derive(Default, Reflect, GizmoConfigGroup)]
struct GridGizmos {}
fn gizmo_grid_system(
    mut gizmos: Gizmos<GridGizmos>,
    editor_windows_2d: Res<resources::EditorWindows2d>,
    camera_query: Query<(&GlobalTransform, &Camera)>,
) {
    // gizmos.line(Vec3::ZERO, Vec3::ONE, Color::BLACK);
    gizmos.arrow(Vec3::ZERO, Vec3::X, Color::RED);
    gizmos.arrow(Vec3::ZERO, Vec3::Y, Color::GREEN);
    gizmos.arrow(Vec3::ZERO, Vec3::Z, Color::BLUE);

    let Some(upper) = editor_windows_2d.windows.get(UPPER_WINDOW) else {
        return;
    };
    let Some(lower) = editor_windows_2d.windows.get(LOWER_WINDOW) else {
        return;
    };

    let Ok((upper_transform, upper_camera)) = camera_query.get(upper.camera) else {
        return;
    };

    let Ok((lower_transform, lower_camera)) = camera_query.get(lower.camera) else {
        return;
    };
    let Some((upper_min, upper_max)) = ortho_view_bounds(upper_camera, upper_transform) else {
        return;
    };
    let Some((lower_min, lower_max)) = ortho_view_bounds(lower_camera, lower_transform) else {
        return;
    };
    if lower.orientation == Orientation2d::Front {
        let ystart = lower_min.y.floor();
        let zstart = upper_min.z.floor();
        let num_lines_yz = (upper_max.z - upper_min.z).max(lower_max.y - lower_min.y) as i32 + 1;

        for yz in 0..num_lines_yz {
            gizmos.line(
                Vec3::new(upper_min.x, yz as f32 + ystart, yz as f32 + zstart),
                Vec3::new(upper_max.x, yz as f32 + ystart, yz as f32 + zstart),
                Color::BLUE,
            );
        }
        let num_lines_x = (upper_max.x - upper_min.x) as i32 + 1;
        let xstart = upper_min.x.floor();
        for x in 0..num_lines_x {
            gizmos.line(
                Vec3::new(x as f32 + xstart, lower_min.y, upper_min.z),
                Vec3::new(x as f32 + xstart, lower_max.y, upper_max.z),
                Color::BLUE,
            );
        }
    } else {
        let xstart = upper_min.x.floor();
        let ystart = lower_min.y.floor();
        let num_lines_xy = (upper_max.x - upper_min.x).max(lower_max.y - lower_min.y) as i32 + 1;

        // gizmos.line(upper_min, upper_max, Color::GREEN);
        // gizmos.line(lower_min, lower_max, Color::YELLOW_GREEN);
        // info!("num_lines: {}", num_lines);
        for xy in 0..num_lines_xy {
            gizmos.line(
                Vec3::new(xy as f32 + xstart, xy as f32 + ystart, upper_min.z),
                Vec3::new(xy as f32 + xstart, xy as f32 + ystart, upper_max.z),
                Color::BLUE,
            );
        }
        let num_lines_z = (upper_max.z - upper_min.z) as i32 + 1;
        let zstart = upper_min.z.floor();
        for z in 0..num_lines_z {
            gizmos.line(
                Vec3::new(upper_min.x, lower_min.y, z as f32 + zstart),
                Vec3::new(upper_max.x, lower_max.y, z as f32 + zstart),
                Color::BLUE,
            );
        }
    }
}

fn setup_grid_gizmos_system(mut config_store: ResMut<GizmoConfigStore>) {
    let (config, _) = config_store.config_mut::<GridGizmos>();
    config.render_layers = render_layers::ortho_views();
    config.line_width = 1.0;
    config.depth_bias = -1.0;
}

pub struct GridPlugin;

impl Plugin for GridPlugin {
    fn build(&self, app: &mut App) {
        app.init_gizmo_group::<GridGizmos>();
        app.add_systems(Update, gizmo_grid_system);
        app.add_systems(Startup, setup_grid_gizmos_system);
    }
}
