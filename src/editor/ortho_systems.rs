use std::collections::BTreeMap;

use bevy::{
    prelude::*,
    render::{
        camera::{Projection, RenderTarget, ScalingMode},
        view::RenderLayers,
    },
    utils::HashMap,
};

use super::{
    components,
    resources::{self, LOWER_WINDOW, UPPER_WINDOW},
    util::{self, Orientation2d, SnapToGrid, WmMouseButton},
};
use crate::{
    csg::{self, PLANE_EPSILON},
    render_layers,
};
// systems related to 2d windows

pub fn setup_editor_system(mut editor_windows_2d: ResMut<resources::EditorWindows2d>) {
    editor_windows_2d.view_max = Vec3::splat(f32::INFINITY);
    editor_windows_2d.view_min = Vec3::splat(f32::NEG_INFINITY);
}

pub fn enter_editor_state(
    wm_state: Res<resources::WmState>,
    mut editor_windows_2d: ResMut<resources::EditorWindows2d>,
    mut commands: Commands,
) {
    let settings_map = if let Ok(file) = std::fs::File::open("windows.yaml") {
        serde_yaml::from_reader(file).unwrap_or_default()
    } else {
        HashMap::<String, resources::EditorWindowSettings>::new()
    };

    let view_configs = vec![
        (
            UPPER_WINDOW,
            Orientation2d::DownFront,
            RenderLayers::layer(render_layers::TOP_2D),
            wm_state.slot_upper2d.offscreen_image.clone(),
        ),
        (
            LOWER_WINDOW,
            Orientation2d::Front,
            RenderLayers::layer(render_layers::SIDE_2D),
            wm_state.slot_lower2d.offscreen_image.clone(),
        ),
    ];
    for (name, t, render_layer, offscreen_image) in view_configs {
        let camera = Camera {
            // target: RenderTarget::Window(window_id),
            target: RenderTarget::Image(offscreen_image.clone()),
            priority: -1,
            ..default()
        };
        // lazy create camera entities
        match editor_windows_2d.windows.entry(name.to_string()) {
            bevy::utils::hashbrown::hash_map::Entry::Vacant(e) => {
                let settings =
                    settings_map
                        .get(name)
                        .cloned()
                        .unwrap_or(resources::EditorWindowSettings {
                            pos_x: 0,
                            pos_y: 0,
                            width: 800,
                            height: 600,
                            orientation: t,
                        });

                let entity = commands
                    .spawn(Camera3dBundle {
                        transform: settings.orientation.get_transform(),
                        camera,
                        projection: Projection::Orthographic(OrthographicProjection {
                            scaling_mode: ScalingMode::FixedHorizontal(10.0),
                            ..default()
                        }),
                        ..default()
                    })
                    .insert(render_layer)
                    .insert(components::Ortho2dCamera)
                    .id();

                e.insert(resources::EditorWindow2d {
                    camera: entity,
                    offscreen_image,
                    settings,
                });
            }

            bevy::utils::hashbrown::hash_map::Entry::Occupied(e) => {
                // entity already exists, just re-attach camera component
                commands.entity(e.get().camera).insert(camera);
            }
        }
    }
}

pub fn leave_editor_state(
    mut commands: Commands,

    query: Query<Entity, With<components::Ortho2dCamera>>,
) {
    for entity in &query {
        commands.entity(entity).remove::<Camera>();
    }
}

pub fn write_window_settings(
    mut last_written_settings: Local<BTreeMap<String, resources::EditorWindowSettings>>,
    editor_windows_2d: Res<resources::EditorWindows2d>,
) {
    let settings = editor_windows_2d
        .windows
        .iter()
        .map(|(name, window)| (name.clone(), window.settings))
        .collect::<BTreeMap<_, _>>();

    if settings != *last_written_settings {
        if let Ok(file) = std::fs::File::create("windows.yaml") {
            let _ = serde_yaml::to_writer(file, &settings);
            *last_written_settings = settings;
            info!("window settings written");
        }
    }
}

pub fn control_input_wm_system(
    // keycodes: Res<Input<KeyCode>>,
    mut editor_windows_2d: ResMut<resources::EditorWindows2d>,
    camera_query: Query<(&GlobalTransform, &Camera)>,
    mut transform_query: Query<&mut Transform>,
    mut event_reader: EventReader<util::WmEvent>,
    mut projection_query: Query<&mut Projection>,
) {
    for event in event_reader.iter() {
        // let focused_name = event.
        // info!("event: {:?}", event);

        match *event {
            util::WmEvent::DragStart {
                window: focused_name,
                button: WmMouseButton::Right,
                pointer_state,
            } => {
                let Some(window) = editor_windows_2d.windows.get(focused_name) else { continue; };
                let Ok((global_transform, camera)) = camera_query.get(window.camera) else {
                    warn!("2d window camera not found: {:?}", window.camera); 
                    continue;
                };
                info!("Right down");
                let Some(ray) = camera.viewport_to_world(global_transform, pointer_state.get_pos_origin_down()) else {
                    warn!("viewport_to_world failed in {}", focused_name); 
                    continue;
                };
                let mut transforms = Vec::new();
                for (_name, window) in &editor_windows_2d.windows {
                    if let Ok(transform) = transform_query.get(window.camera) {
                        transforms.push((window.camera, *transform));
                    }
                }

                editor_windows_2d.translate_drag = Some(resources::TranslateDrag {
                    start_ray: ray,
                    start_focus: focused_name.to_string(),
                    start_global_transform: *global_transform,
                    start_transforms: transforms,
                });
            }
            util::WmEvent::DragUpdate {
                window: focused_name,
                button: WmMouseButton::Right,
                pointer_state,
            } => {
                let Some(window) = editor_windows_2d.windows.get(focused_name) else { continue; };
                let Ok((_global_transform, camera)) = camera_query.get(window.camera) else {
                    warn!("2d window camera not found: {:?}", window.camera); 
                    continue;
                };
                let mut transforms = Vec::new();
                for (_name, window) in &editor_windows_2d.windows {
                    if let Ok(transform) = transform_query.get(window.camera) {
                        transforms.push((window.camera, *transform));
                    }
                }
                if let Some(resources::TranslateDrag {
                    start_ray,
                    start_focus: _,
                    start_global_transform,
                    start_transforms,
                }) = &editor_windows_2d.translate_drag
                {
                    let Some(ray) = camera.viewport_to_world(start_global_transform, pointer_state.get_pos_origin_down()) else {
                        warn!("viewport_to_world failed in {}", focused_name); 
                        continue;
                    };
                    let d = start_ray.origin - ray.origin;
                    info!(
                        "translate drag update: {:?} {:?}",
                        start_ray.origin, ray.origin
                    );
                    for (entity, start_transform) in start_transforms {
                        if let Ok(mut transform) = transform_query.get_mut(*entity) {
                            transform.translation = start_transform.translation + d;
                        }
                    }
                }
            }
            util::WmEvent::DragEnd {
                window: _,
                button: WmMouseButton::Right,
                pointer_state: _,
            } => {
                info!("translate drag end");
                editor_windows_2d.translate_drag = None;
            }
            util::WmEvent::ZoomDelta(zoom_delta) => {
                for (_name, window) in &editor_windows_2d.windows {
                    let Ok(mut projection) = projection_query.get_mut(window.camera) else {
                        warn!("2d window camera transform / projection not found: {:?}", window.camera); 
                        continue;
                    };

                    let Projection::Orthographic(ortho) = &mut *projection else {
                        warn!("2d window camera has not ortho projection: {:?}", window.camera); 
                        continue;
                    };

                    let ScalingMode::FixedHorizontal(scaling) = &mut ortho.scaling_mode else {
                        warn!("2d window camera has not ortho projection: {:?}", window.camera); 
                        continue;
                    };

                    if *scaling * zoom_delta > 0.0 {
                        *scaling *= zoom_delta;
                    }
                }
            }
            _ => (),
        }
    }
}

pub fn adjust_clip_planes_system(
    mut editor_windows_2d: ResMut<resources::EditorWindows2d>,
    mut camera_query: Query<(&GlobalTransform, &Camera, &mut Projection, &mut Transform)>,
) {
    let editor_windows_2d = &mut *editor_windows_2d;

    let Some(upper) = editor_windows_2d.windows.get(UPPER_WINDOW) else {
        return;
    };
    let Some(lower) = editor_windows_2d.windows.get(LOWER_WINDOW) else {
        return;
    };

    let Ok((upper_transform, upper_camera, _upper_projection, _)) = camera_query.get(upper.camera) else {
        return;
    };

    let Ok((lower_transform, lower_camera, _lower_projection, _)) = camera_query.get(lower.camera) else {
        return;
    };

    let Some((Some(upper_min), Some(upper_max))) =
        upper_camera.logical_viewport_rect().map(|(min, max)| {
            (
                upper_camera.viewport_to_world(upper_transform, min),
                upper_camera.viewport_to_world(upper_transform, max),
            )
        }) else {
            return;
        };

    let Some((Some(lower_min), Some(lower_max))) =
        lower_camera.logical_viewport_rect().map(|(min, max)| {
            (
                lower_camera.viewport_to_world(lower_transform, min),
                lower_camera.viewport_to_world(lower_transform, max),
            )
        }) else {
            return;
        };

    // FIXME: this is all pretty much hardcoded to the 'Right' view
    // info!("upper bounds: {:?} {:?}", upper_min, upper_max);
    // info!("lower bounds: {:?} {:?}", lower_min, lower_max);

    {
        let xmin = upper_min.origin.x;
        let xmax = upper_max.origin.x;

        let Ok((_, _, mut lower_projection, mut lower_transform)) = camera_query.get_mut(lower.camera) else {
            return;
        };
        let Projection::Orthographic(lower_ortho) = &mut *lower_projection else {
            return;
        };

        lower_transform.translation.x = xmin;
        lower_ortho.far = xmax - xmin;

        editor_windows_2d.view_max.x = xmax;
        editor_windows_2d.view_min.x = xmin;
    }

    {
        let ymin = lower_min.origin.y;
        let ymax = lower_max.origin.y;

        let Ok((_, _, mut upper_projection, mut upper_transform)) = camera_query.get_mut(upper.camera) else {
            return;
        };
        let Projection::Orthographic(upper_ortho) = &mut *upper_projection else {
            return;
        };

        upper_transform.translation.y = ymax;
        upper_ortho.far = ymax - ymin;

        editor_windows_2d.view_max.y = ymax;
        editor_windows_2d.view_min.y = ymin;
    }

    // info!("far: {}", lower_ortho.far);

    // let Projection::Orthographic(upper_ortho) = &mut *upper_projection else {
    //     return
    // };
    // let Projection::Orthographic(lower_ortho) = &mut *lower_projection else {
    //     return
    // };

    // upper.camera
    // info!("upper: {:?} {:?}", upper_projection,);
}

#[allow(clippy::too_many_arguments)]
pub fn edit_input_system(
    mut commands: Commands,
    mut event_reader: EventReader<util::WmEvent>,
    selection: Res<resources::Selection>,
    keycodes: Res<Input<KeyCode>>,
    editor_windows_2d: Res<resources::EditorWindows2d>,

    camera_query: Query<(&GlobalTransform, &Camera)>,
    brush_query: Query<&csg::Brush, Without<components::DragAction>>,
    point_query: Query<
        &Transform,
        (
            With<components::EditablePoint>,
            Without<components::DragAction>,
        ),
    >,
    mut brush_drag_query: Query<
        (
            Entity,
            &components::DragAction,
            &mut csg::Brush,
            &mut components::CsgRepresentation,
        ),
        Without<components::EditablePoint>,
    >,
    mut point_drag_query: Query<
        (Entity, &components::DragAction, &mut Transform),
        (With<components::EditablePoint>),
    >,
    // mut transform_query: Query<&mut Transform>,
) {
    for event in event_reader.iter() {
        debug!("event edit: {:?}", event);
        match *event {
            util::WmEvent::DragStart {
                window: focused_name,
                button: util::WmMouseButton::Left,
                pointer_state,
            } => {
                let Some(window) = editor_windows_2d.windows.get(focused_name) else { continue; };
                let Ok((global_transform, camera)) = camera_query.get(window.camera) else {
                    warn!("2d window camera not found: {:?}", window.camera); 
                    continue;
                };
                info!("left down");
                let Some(ray) = camera.viewport_to_world(global_transform, pointer_state.get_pos_origin_down()) else {
                    warn!("viewport_to_world failed in {}", focused_name); 
                    continue;
                };

                info!("click ray {}: {:?}", focused_name, ray);

                if let Some(primary) = selection.primary {
                    // match brush_query.get(primary) {
                    if let Ok(brush) = brush_query.get(primary) {
                        let affected_faces = brush.get_planes_behind_ray(ray);

                        if !affected_faces.is_empty() {
                            commands.entity(primary).insert(components::DragAction {
                                start_ray: ray,
                                action: components::DragActionType::Face { affected_faces },
                            });
                            info!("start face drag for {:?}", primary); // the crowd put on their affected_faces as The Iron Sheik did his signature face-drag on el Pollo Loco
                        } else {
                            let affected_faces = brush
                                .planes
                                .iter()
                                .enumerate()
                                .map(|(i, face)| (i, face.w))
                                .collect();
                            commands.entity(primary).insert(components::DragAction {
                                start_ray: ray,
                                action: components::DragActionType::WholeBrush { affected_faces },
                            });
                            info!("start whole-brush drag for {:?}", primary);
                        }
                    } else if let Ok(transform) = point_query.get(primary) {
                        info!("light drag start");

                        commands.entity(primary).insert(components::DragAction {
                            start_ray: ray,
                            action: components::DragActionType::NonBrush {
                                start_translation: transform.translation,
                            },
                        });
                    }
                }
            }
            util::WmEvent::DragUpdate {
                window: focused_name,
                button: util::WmMouseButton::Left,
                pointer_state,
            } => {
                let Some(window) = editor_windows_2d.windows.get(focused_name) else { continue; };
                let Ok((global_transform, camera)) = camera_query.get(window.camera) else {
                    warn!("2d window camera not found: {:?}", window.camera); 
                    continue;
                };
                // info!("left down");
                let Some(ray) = camera.viewport_to_world(global_transform, pointer_state.get_pos_origin_down()) else {
                    warn!("viewport_to_world failed in {}", focused_name); 
                    continue;
                };

                // update dragged objects. Do this in two steps, only touch EditorObject as mutable if there is a relevant change
                // to prevent triggering the bevy change detection.
                let mut csg_updates = Vec::new();
                let mut transform_updates = Vec::new();
                for (entity, drag_action, brush, _) in &brush_drag_query {
                    let drag_delta = ray.origin - drag_action.start_ray.origin;

                    debug!("drag: {:?} on brush {:?}", drag_delta, entity);

                    match &drag_action.action {
                        components::DragActionType::Face { affected_faces }
                        | components::DragActionType::WholeBrush { affected_faces } /* yay, free implementation */ => {
                            let mut new_brush = brush.clone();
                            let mut relevant_change = false;
                            for (face, start_w) in affected_faces {
                                let normal = brush.planes[*face].normal;

                                let d = drag_delta.dot(normal);

                                let snap = if keycodes.pressed(KeyCode::LAlt) {
                                    0.5
                                } else {
                                    0.1
                                };
                                // let d_snap = (d / snap).round() * snap;

                                let new_w = (*start_w + d).snap(snap);

                                // compare to the current w of the plane, only apply new value if it changed
                                let current_w = brush.planes[*face].w;
                                if (new_w - current_w).abs() < PLANE_EPSILON {
                                    continue;
                                }
                                new_brush.planes[*face].w = new_w;
                                relevant_change = true;
                            }
                            if relevant_change {
                                let csg: Result<csg::Csg, _> = new_brush.clone().try_into();
                                match csg {
                                    Ok(csg) => {
                                        let (center, radius) = csg.bounding_sphere();
                                        csg_updates.push((
                                            entity,
                                            (
                                                new_brush,
                                                components::CsgRepresentation {
                                                    center,
                                                    radius,
                                                    csg,
                                                },
                                            ),
                                        ));
                                    }
                                    Err(_) => {
                                        warn!("edit action degenerates brush. ignoring.");
                                    }
                                }
                            }
                        }
                        _ => warn!( "invalid drag action in brush object"),
                    }
                }

                for (entity, drag_action, transform) in &point_drag_query {
                    let drag_delta = ray.origin - drag_action.start_ray.origin;

                    debug!("drag: {:?} on point {:?}", drag_delta, entity);

                    match &drag_action.action {
                        components::DragActionType::NonBrush { start_translation } => {
                            transform_updates.push((entity, *start_translation + drag_delta));
                        }
                        _ => warn!("invalid drag action in editable point."),
                    }
                }

                // info!("updates: {:?}", updates);

                for (entity, (obj, bounds)) in csg_updates {
                    info!("apply update on {:?}", entity);
                    if let Ok((_, _, mut target_obj, mut target_bounds)) =
                        brush_drag_query.get_mut(entity)
                    {
                        *target_obj = obj;
                        *target_bounds = bounds;
                    }
                }

                for (entity, translation) in transform_updates {
                    if let Ok((_, _, mut transform)) = point_drag_query.get_mut(entity) {
                        transform.translation = translation;
                    }
                }
            }

            util::WmEvent::DragEnd {
                window: _,
                button: util::WmMouseButton::Left,
                pointer_state: _,
            } => {
                for entity in brush_drag_query
                    .iter()
                    .map(|(e, _, _, _)| e)
                    .chain(point_drag_query.iter().map(|(e, _, _)| e))
                {
                    commands.entity(entity).remove::<components::DragAction>();
                    info!("stop drag for {:?}", entity);
                }
            }
            _ => (),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn select_input_system(
    mut event_reader: EventReader<util::WmEvent>,
    mut selection: ResMut<resources::Selection>,
    editor_windows_2d: Res<resources::EditorWindows2d>,
    camera_query: Query<(&GlobalTransform, &Camera)>,
    brush_query: Query<(Entity, &csg::Brush, &components::CsgRepresentation)>,
    point_query: Query<(Entity, &Transform), With<components::EditablePoint>>,
) {
    for event in event_reader.iter() {
        if let util::WmEvent::Clicked {
            window: focused_name,
            button: util::WmMouseButton::Left,
            pointer_state,
        } = *event
        {
            let Some(window) = editor_windows_2d.windows.get(focused_name) else { continue };
            let Ok((global_transform, camera)) = camera_query.get(window.camera) else {
                warn!("2d window camera not found: {:?}", window.camera);
                continue;
            };

            let Some(ray) = camera.viewport_to_world(global_transform, pointer_state.get_pos_origin_down()) else {
                warn!("viewport_to_world failed in {}", focused_name); 
                continue;
            };

            // editor_windows_2d.
            let brush_selection = brush_query.iter().filter_map(|(entity, brush, csg)| {
                if !csg.csg.polygons.iter().any(|poly| {
                    poly.vertices.iter().any(|v| {
                        v.position.x >= editor_windows_2d.view_min.x
                            && v.position.y >= editor_windows_2d.view_min.y
                            && v.position.z >= editor_windows_2d.view_min.z
                            && v.position.x <= editor_windows_2d.view_max.x
                            && v.position.y <= editor_windows_2d.view_max.y
                            && v.position.z <= editor_windows_2d.view_max.z
                    })
                }) {
                    return None;
                }

                let affected_faces = brush.get_planes_behind_ray(ray);
                if affected_faces.is_empty() {
                    Some(entity)
                } else {
                    None
                }
            });
            // .collect::<Vec<_>>();

            let point_selection = point_query.iter().filter_map(|(entity, transform)| {
                let pos = transform.translation;
                if pos.x >= editor_windows_2d.view_min.x
                    && pos.y >= editor_windows_2d.view_min.y
                    && pos.z >= editor_windows_2d.view_min.z
                    && pos.x <= editor_windows_2d.view_max.x
                    && pos.y <= editor_windows_2d.view_max.y
                    && pos.z <= editor_windows_2d.view_max.z
                    && distance(ray, pos) < 0.1
                {
                    Some(entity)
                } else {
                    None
                }
            });

            let selection_set = brush_selection.chain(point_selection).collect::<Vec<_>>();

            info!("selection set: {:?}", selection_set);

            if selection_set != selection.last_set {
                selection.last_set = selection_set;
                selection.last_set_index = 0;
            } else {
                selection.last_set_index += 1;
            }

            if !selection.last_set.is_empty() {
                selection.primary =
                    Some(selection.last_set[selection.last_set_index % selection.last_set.len()]);
            }
        }
    }
}

// https://mathworld.wolfram.com/Point-LineDistance3-Dimensional.html
fn distance(ray: Ray, x0: Vec3) -> f32 {
    let x1 = ray.origin;
    let x2 = ray.origin + ray.direction;
    (x0 - x1).cross(x0 - x2).length() / ray.direction.length()
}
