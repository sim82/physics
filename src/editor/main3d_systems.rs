use bevy::prelude::*;
use bevy_inspector_egui::egui::plot::Polygon;

use crate::csg;

use super::{components, resources, util};

#[allow(clippy::too_many_arguments)]
pub fn select_input_system(
    mut commands: Commands,
    mut event_reader: EventReader<util::WmEvent>,
    mut selection: ResMut<resources::Selection>,
    editor_windows_2d: Res<resources::EditorWindows2d>,
    material_browser: Res<resources::MaterialBrowser>,
    camera_query: Query<(&GlobalTransform, &Camera), With<components::Main3dCamera>>,
    mut processed_csg_query: Query<(
        Entity,
        &mut components::BrushMaterialProperties,
        &components::ProcessedCsg,
    )>,
    point_query: Query<(Entity, &Transform), With<components::EditablePoint>>,
    selected_query: Query<Entity, With<components::Selected>>,
) {
    for event in event_reader.iter() {
        if let util::WmEvent::Clicked {
            window: focused_name,
            button: util::WmMouseButton::Left,
            pointer_state,
        } = *event
        {
            if focused_name != resources::MAIN3D_WINDOW {
                continue;
            }
            info!("event: {:?}", event);

            let Ok((global_transform, camera)) = camera_query.get_single() else {
                warn!("3d window camera not found");
                continue;
            };

            let Some(ray) = camera.viewport_to_world(global_transform, pointer_state.get_pos_origin_down()) else {
                warn!("viewport_to_world failed in {}", focused_name); 
                continue;
            };

            info!("3d ray: {:?}", ray);

            // let brush_selection = brush_query
            //     .iter()
            //     .filter_map(|(entity, _brush, processed_csg)| {
            //         for polygon in processed_csg.bsp.all_polygons() {}
            //         for tri in csg.csg.get_triangles() {
            //             // check against view bounds to only include visible brushes
            //             // if !tri.0.iter().any(|v| editor_windows_2d.in_view_bounds(v)) {
            //             //     continue;
            //             // }
            //             if util::raycast_moller_trumbore(&ray, &tri.0) {
            //                 return Some(entity);
            //             }
            //         }
            //         None
            //     })
            //     .collect::<Vec<_>>();

            // find clicked face
            for (entity, mut material_properties, processed_csg) in &mut processed_csg_query {
                'poly_loop: for polygon in processed_csg.bsp.all_polygons() {
                    let mut res = Vec::new();
                    polygon.get_triangles(&mut res);
                    for (tri, normal, appearance) in res {
                        if util::raycast_moller_trumbore(&ray, &tri, true) {
                            info!("hit {:?} in {:?}", polygon, entity);
                            if !material_browser.selected_material.is_empty() {
                                material_properties.materials[appearance as usize] =
                                    material_browser.selected_material.clone();
                                info!(
                                    "assign material: {} {}",
                                    appearance, material_browser.selected_material
                                );
                            }

                            break 'poly_loop;
                        }
                    }
                }
            }

            // info!("brushes: {:?}", brush_selection);
        }
    }
}
