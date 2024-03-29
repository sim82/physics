use bevy::{
    prelude::*,
    render::{
        camera::{Projection, RenderTarget, ScalingMode},
        view::RenderLayers,
    },
    utils::HashSet,
};

use super::{
    components,
    edit_commands::EditCommands,
    resources::{self, LOWER_WINDOW, UPPER_WINDOW},
    util::{self, Orientation2d, SnapToGrid, WmMouseButton},
};
use crate::{
    edit_commands::{update_brush_drag, update_point_transform},
    util::ortho_view_bounds,
};

use shared::render_layers;

use csg::PLANE_EPSILON;
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
            order: -1,
            ..default()
        };
        // lazy create camera entities
        #[allow(unused)] // this is all a mess anyways
        match editor_windows_2d.windows.entry(name.to_string()) {
            bevy::utils::hashbrown::hash_map::Entry::Vacant(e) => {
                let camera = commands
                    .spawn(Camera3dBundle {
                        transform: t.get_transform(),
                        camera,
                        projection: Projection::Orthographic(OrthographicProjection {
                            scaling_mode: ScalingMode::FixedHorizontal(10.0),
                            ..default()
                        }),
                        ..default()
                    })
                    .insert(render_layer)
                    .insert(components::Ortho2dCamera)
                    .insert(bevy::core_pipeline::fxaa::Fxaa {
                        enabled: false,
                        ..default()
                    })
                    .id();

                let grid_entity = { commands.spawn((Name::new(format!("{} grid", name)),)).id() };
                e.insert(resources::EditorWindow2d {
                    camera,
                    grid: grid_entity,
                    offscreen_image,
                    orientation: t,
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

pub fn control_input_wm_system(
    // keycodes: Res<Input<KeyCode>>,
    mut editor_windows_2d: ResMut<resources::EditorWindows2d>,
    camera_query: Query<(&GlobalTransform, &Camera)>,
    mut transform_query: Query<&mut Transform>,
    mut event_reader: EventReader<util::WmEvent>,
    mut projection_query: Query<&mut Projection>,
) {
    for event in event_reader.read() {
        // let focused_name = event.
        // info!("event: {:?}", event);

        match *event {
            util::WmEvent::DragStart {
                window: focused_name,
                button: WmMouseButton::Right,
                pointer_state,
            } => {
                let Some(window) = editor_windows_2d.windows.get(focused_name) else {
                    continue;
                };
                let Ok((global_transform, camera)) = camera_query.get(window.camera) else {
                    warn!("2d window camera not found: {:?}", window.camera);
                    continue;
                };
                info!("Right down");
                let Some(ray) =
                    camera.viewport_to_world(global_transform, pointer_state.get_pos_origin_down())
                else {
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
                let Some(window) = editor_windows_2d.windows.get(focused_name) else {
                    continue;
                };
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
                    let Some(ray) = camera.viewport_to_world(
                        start_global_transform,
                        pointer_state.get_pos_origin_down(),
                    ) else {
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
                        warn!(
                            "2d window camera transform / projection not found: {:?}",
                            window.camera
                        );
                        continue;
                    };

                    let Projection::Orthographic(ortho) = &mut *projection else {
                        warn!(
                            "2d window camera has not ortho projection: {:?}",
                            window.camera
                        );
                        continue;
                    };

                    let ScalingMode::FixedHorizontal(scaling) = &mut ortho.scaling_mode else {
                        warn!(
                            "2d window camera has not ortho projection: {:?}",
                            window.camera
                        );
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
    keycodes: Res<ButtonInput<KeyCode>>,

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

    let upper_orientation = &upper.orientation;
    let lower_orientation = &lower.orientation;

    let Ok((upper_transform, upper_camera, upper_projection, _)) = camera_query.get(upper.camera)
    else {
        return;
    };

    let Ok((lower_transform, lower_camera, _lower_projection, _)) = camera_query.get(lower.camera)
    else {
        return;
    };

    let scaling = if let Projection::Orthographic(OrthographicProjection {
        scaling_mode: ScalingMode::FixedHorizontal(scaling),
        ..
    }) = upper_projection
    {
        *scaling
    } else {
        warn!("failed to get scaling factor from ortographic projection");
        1.0
    };

    // meh I guess there is a formula for that...
    let mut trunc_scaling = 1.0;
    while trunc_scaling * 2.0 < scaling {
        trunc_scaling *= 10.0
    }
    // info!("scaling: {} -> {}", scaling, trunc_scaling);

    let Some((upper_min, upper_max)) = ortho_view_bounds(upper_camera, upper_transform) else {
        return;
    };
    let Some((lower_min, lower_max)) = ortho_view_bounds(lower_camera, lower_transform) else {
        return;
    };

    {
        let min = upper_orientation.get_up_axis(upper_min) - 5.0;
        let max = upper_orientation.get_up_axis(upper_max) + 5.0;

        let Ok((_, _, mut lower_projection, mut lower_transform)) =
            camera_query.get_mut(lower.camera)
        else {
            return;
        };
        let Projection::Orthographic(lower_ortho) = &mut *lower_projection else {
            return;
        };

        *upper_orientation.get_up_axis_mut(&mut lower_transform.translation) = max;
        lower_ortho.far = max - min;
        // info!("depth: {}", lower_ortho.far);
        *upper_orientation.get_up_axis_mut(&mut editor_windows_2d.view_max) = max;
        *upper_orientation.get_up_axis_mut(&mut editor_windows_2d.view_min) = min;
    }

    {
        let min = lower_orientation.get_up_axis(lower_min) - 5.0;
        let max = lower_orientation.get_up_axis(lower_max) + 5.0;

        let Ok((_, _, mut upper_projection, mut upper_transform)) =
            camera_query.get_mut(upper.camera)
        else {
            return;
        };
        let Projection::Orthographic(upper_ortho) = &mut *upper_projection else {
            return;
        };

        *lower_orientation.get_up_axis_mut(&mut upper_transform.translation) = max;

        upper_ortho.far = max - min;
        *lower_orientation.get_up_axis_mut(&mut editor_windows_2d.view_max) = max;
        *lower_orientation.get_up_axis_mut(&mut editor_windows_2d.view_min) = min;
    }

    if keycodes.just_pressed(KeyCode::F2) {
        let mut right = 0.0;
        if let Some(window) = editor_windows_2d.windows.get_mut(resources::UPPER_WINDOW) {
            window.orientation = window.orientation.flipped();
            if let Ok((_, _, _, mut transform)) = camera_query.get_mut(window.camera) {
                transform.rotation = window.orientation.get_transform().rotation;
                right = window.orientation.get_right_axis(transform.translation);
            };
        }
        if let Some(window) = editor_windows_2d.windows.get_mut(resources::LOWER_WINDOW) {
            window.orientation = window.orientation.flipped();
            if let Ok((_, _, _, mut transform)) = camera_query.get_mut(window.camera) {
                transform.rotation = window.orientation.get_transform().rotation;
                *window
                    .orientation
                    .get_right_axis_mut(&mut transform.translation) = right;
            };
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn edit_input_system(
    mut edit_commands: EditCommands,
    mut commands: Commands,
    mut event_reader: EventReader<util::WmEvent>,
    keycodes: Res<ButtonInput<KeyCode>>,
    editor_windows_2d: Res<resources::EditorWindows2d>,

    camera_query: Query<(&GlobalTransform, &Camera)>,
    brush_query: Query<&csg::Brush, Without<components::DragAction>>,
    brush_drag_query: Query<
        (
            Entity,
            &components::DragAction,
            &csg::Brush,
            &components::CsgRepresentation,
        ),
        Without<components::EditablePoint>,
    >,
    point_drag_query: Query<(Entity, &components::DragAction), With<components::EditablePoint>>,
    selected_query: Query<Entity, With<components::Selected>>,
) {
    for event in event_reader.read() {
        debug!("event edit: {:?}", event);
        match *event {
            util::WmEvent::DragStart {
                window: focused_name,
                button: util::WmMouseButton::Left,
                pointer_state,
            } => {
                let Some(window) = editor_windows_2d.windows.get(focused_name) else {
                    continue;
                };
                let Ok((global_transform, camera)) = camera_query.get(window.camera) else {
                    warn!("2d window camera not found: {:?}", window.camera);
                    continue;
                };
                info!("left down");
                let Some(ray) =
                    camera.viewport_to_world(global_transform, pointer_state.get_pos_origin_down())
                else {
                    warn!("viewport_to_world failed in {}", focused_name);
                    continue;
                };

                info!("click ray {}: {:?}", focused_name, ray);

                if let Ok(primary) = selected_query.get_single() {
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
                    } else if let Ok(transform) = edit_commands.transform_query.get(primary) {
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
                let Some(window) = editor_windows_2d.windows.get(focused_name) else {
                    continue;
                };
                let Ok((global_transform, camera)) = camera_query.get(window.camera) else {
                    warn!("2d window camera not found: {:?}", window.camera);
                    continue;
                };
                // info!("left down");
                let Some(ray) =
                    camera.viewport_to_world(global_transform, pointer_state.get_pos_origin_down())
                else {
                    warn!("viewport_to_world failed in {}", focused_name);
                    continue;
                };

                // update dragged objects. Do this in two steps, only touch EditorObject as mutable if there is a relevant change
                // to prevent triggering the bevy change detection.
                // let mut csg_updates = Vec::new();
                for (entity, drag_action, brush, _) in &brush_drag_query {
                    let drag_delta = ray.origin - drag_action.start_ray.origin;
                    debug!("drag: {:?} on brush {:?}", drag_delta, entity);

                    // apply grid-snapping to drag-delta
                    let snap = if keycodes.pressed(KeyCode::AltLeft) {
                        0.5
                    } else {
                        0.1
                    };
                    let drag_delta = drag_delta.snap(snap);

                    match &drag_action.action {
                        components::DragActionType::Face { affected_faces }
                        | components::DragActionType::WholeBrush { affected_faces } => {
                            let mut new_brush = brush.clone();
                            let mut relevant_change = false;
                            for (face, start_w) in affected_faces {
                                // 'moving' a plane works by projecting the drag-delta onto the normal of the
                                // plane to get the component of the drag relevant for the current plane.
                                // The dot-product between the normal and its projection equals the distance
                                // we need to move the plane along its normal, so we just can add this to the
                                // w component of the plane.
                                //
                                // Grid-snapping works by snapping the drag-delta before this whole process.

                                let normal = brush.planes[*face].normal;

                                let projected = drag_delta.project_onto(normal);
                                let new_w = start_w + projected.dot(normal);

                                // compare to the current w of the plane, only apply new value if it changed
                                let current_w = brush.planes[*face].w;
                                if (new_w - current_w).abs() < PLANE_EPSILON {
                                    continue;
                                }
                                new_brush.planes[*face].w = new_w;
                                relevant_change = true;
                            }
                            if relevant_change {
                                let res = edit_commands.apply(update_brush_drag::Command {
                                    entity,
                                    start_brush: brush.clone(),
                                    brush: new_brush,
                                });
                                if let Err(err) = res {
                                    warn!("update_brush_drag apply failed: {:?}", err);
                                }
                            }
                        }
                        _ => warn!("invalid drag action in brush object"),
                    }
                }

                for (entity, drag_action) in &point_drag_query {
                    let drag_delta = ray.origin - drag_action.start_ray.origin;

                    debug!("drag: {:?} on point {:?}", drag_delta, entity);

                    match &drag_action.action {
                        components::DragActionType::NonBrush { start_translation } => {
                            let res = edit_commands.apply(update_point_transform::Command {
                                entity,
                                transform: Transform::from_translation(
                                    (*start_translation + drag_delta).snap(0.1),
                                ),
                            });
                            if let Err(err) = res {
                                warn!("update_point_transform apply failed: {:?}", err);
                            }
                        }
                        _ => warn!("invalid drag action in editable point."),
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
                    .chain(point_drag_query.iter().map(|(e, _)| e))
                {
                    edit_commands.end_drag(entity);
                    info!("stop drag for {:?}", entity);
                }
            }
            _ => (),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn select_input_system(
    mut commands: Commands,
    mut event_reader: EventReader<util::WmEvent>,
    mut selection: ResMut<resources::SelectionPickSet>,
    editor_windows_2d: Res<resources::EditorWindows2d>,
    camera_query: Query<(&GlobalTransform, &Camera)>,
    brush_query: Query<(Entity, &csg::Brush, &components::CsgRepresentation)>,
    point_query: Query<(Entity, &Transform), With<components::EditablePoint>>,
    selected_query: Query<Entity, With<components::Selected>>,
) {
    for event in event_reader.read() {
        if let util::WmEvent::Clicked {
            window: focused_name,
            button: util::WmMouseButton::Left,
            pointer_state,
        } = *event
        {
            if pointer_state.modifiers.alt {
                continue;
            }
            info!("event: {:?}", event);

            info!(
                "focused: {} {:?}",
                focused_name,
                editor_windows_2d.windows.keys()
            );
            let Some(window) = editor_windows_2d.windows.get(focused_name) else {
                warn!("no 2d window focused");
                continue;
            };
            let Ok((global_transform, camera)) = camera_query.get(window.camera) else {
                warn!("2d window camera not found: {:?}", window.camera);
                continue;
            };

            let Some(ray) =
                camera.viewport_to_world(global_transform, pointer_state.get_pos_origin_down())
            else {
                warn!("viewport_to_world failed in {}", focused_name);
                continue;
            };

            info!("brute force selection");
            // TODO: brute force raycast against all brushes. can be accelerated by spatial index if necessary
            let brush_selection = brush_query.iter().filter_map(|(entity, _brush, csg)| {
                for tri in csg.csg.get_triangles() {
                    info!("select check {entity:?}");
                    // check against view bounds to only include visible brushes
                    if !tri.0.iter().any(|v| editor_windows_2d.in_view_bounds(v)) {
                        continue;
                    }
                    if util::raycast_moller_trumbore(&ray, &tri.0, false).is_some() {
                        return Some(entity);
                    }
                }
                None
            });

            let point_selection = point_query.iter().filter_map(|(entity, transform)| {
                let pos = transform.translation;
                if editor_windows_2d.in_view_bounds(&pos)
                    && util::ray_point_distance(ray, pos) < 0.2
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

            let mut primary_selection = None;
            if !selection.last_set.is_empty() {
                primary_selection =
                    Some(selection.last_set[selection.last_set_index % selection.last_set.len()]);
            }

            let old_selection = selected_query.iter().collect::<HashSet<_>>();

            for entity in &old_selection {
                if Some(*entity) != primary_selection {
                    commands.entity(*entity).remove::<components::Selected>();
                }
            }
            if let Some(entity) = &primary_selection {
                commands.entity(*entity).insert(components::Selected);
            }
        }
    }
}
