use bevy::prelude::*;

use super::{components, edit_commands::EditCommands, resources, util};
use crate::edit_commands::set_brush_material;

#[allow(clippy::too_many_arguments)]
pub fn select_input_system(
    mut edit_commands: EditCommands,
    mut event_reader: EventReader<util::WmEvent>,
    mut material_browser: ResMut<resources::MaterialBrowser>,
    camera_query: Query<(&GlobalTransform, &Camera), With<components::Main3dCamera>>,
    processed_csg_query: Query<(Entity, &components::ProcessedCsg)>,
) {
    for event in event_reader.iter() {
        if let util::WmEvent::Clicked {
            window: focused_name,
            button,
            pointer_state,
        } = *event
        {
            if focused_name != resources::MAIN3D_WINDOW {
                continue;
            }
            if !matches!(
                button,
                util::WmMouseButton::Left | util::WmMouseButton::Right
            ) {
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

            // find clicked face
            let mut closest_hit = None;
            let mut closest_hit_distance = std::f32::INFINITY;
            for (entity, processed_csg) in &processed_csg_query {
                'poly_loop: for polygon in processed_csg.bsp.all_polygons() {
                    let mut res = Vec::new();
                    polygon.get_triangles(&mut res);
                    for (tri, _normal, appearance) in res {
                        if let Some(hit) = util::raycast_moller_trumbore(&ray, &tri, true) {
                            // info!("hit {:?} in {:?}", polygon, entity);
                            if hit.distance < closest_hit_distance {
                                closest_hit = Some((entity, appearance));
                                closest_hit_distance = hit.distance;
                                break 'poly_loop; // cannot hit another poly from same brush, since they are convex
                            }
                        }
                    }
                }
            }

            if let Some((entity, appearance)) = closest_hit {
                if button == util::WmMouseButton::Left
                    && !material_browser.selected_material.is_empty()
                {
                    let _ = edit_commands.apply(set_brush_material::Command {
                        entity,
                        face: appearance,
                        material: material_browser.selected_material.clone(),
                    });
                    // material_properties.materials[appearance as usize] =
                    //     material_browser.selected_material.clone();
                    info!(
                        "assign material: {} {}",
                        appearance, material_browser.selected_material
                    );
                } else if button == util::WmMouseButton::Right {
                    if let Ok((material_properties, _)) = edit_commands.brush_query.get(entity) {
                        material_browser.selected_material =
                            material_properties.materials[appearance as usize].clone();
                        info!(
                            "select material: {} {}",
                            appearance, material_browser.selected_material
                        );
                    }
                }
            }

            // info!("brushes: {:?}", brush_selection);
        }
    }
}
