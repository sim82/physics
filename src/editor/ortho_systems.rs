use std::collections::BTreeMap;

use bevy::{
    input::{
        mouse::{MouseButtonInput, MouseWheel},
        ButtonState,
    },
    prelude::*,
    render::{
        camera::{Projection, RenderTarget, ScalingMode},
        view::RenderLayers,
    },
    utils::HashMap,
    window::{CreateWindow, WindowFocused, WindowId, WindowResized},
};

use super::{
    components::{self, EditorObject},
    resources::{self, EditorWindowSettings, Selection, TranslateDrag, LOWER_WINDOW, UPPER_WINDOW},
    util::{self, Orientation2d},
};
use crate::{
    csg::{self, PLANE_EPSILON},
    editor::{
        components::{CsgRepresentation, DragAction, DragActionType},
        util::{SnapToGrid, WmMouseButton},
    },
    render_layers,
};
// systems related to 2d windows

pub fn setup_editor_window(
    wm_state: Res<resources::WmState>,
    mut editor_windows_2d: ResMut<resources::EditorWindows2d>,
    mut commands: Commands,
    mut create_window_events: EventWriter<CreateWindow>,
) {
    // FIXME: this whole function looks a bit goofy...

    let settings_map = if let Ok(file) = std::fs::File::open("windows.yaml") {
        serde_yaml::from_reader(file).unwrap_or_default()
    } else {
        HashMap::<String, EditorWindowSettings>::new()
    };

    let mut transforms = vec![
        (
            UPPER_WINDOW,
            None,
            Orientation2d::DownFront,
            RenderLayers::layer(render_layers::TOP_2D),
            wm_state.slot_upper2d.offscreen_image.clone(),
            // Transform::from_xyz(0.0, 6.0, 0.0).looking_at(Vec3::ZERO, Vec3::X),
        ),
        (
            LOWER_WINDOW,
            None,
            Orientation2d::Front,
            RenderLayers::layer(render_layers::SIDE_2D),
            wm_state.slot_lower2d.offscreen_image.clone(),
            // Transform::from_xyz(-6.0, 0.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
        ),
    ];
    for (i, (name, window2d, t, render_layer, offscreen_image)) in transforms.iter_mut().enumerate()
    {
        let settings = settings_map
            .get(*name)
            .cloned()
            .unwrap_or(EditorWindowSettings {
                pos_x: 0,
                pos_y: 0,
                width: 800,
                height: 600,
                orientation: *t,
            });

        let window_id = WindowId::new();

        // sends out a "CreateWindow" event, which will be received by the windowing backend
        // create_window_events.send(CreateWindow {
        //     id: window_id,
        //     descriptor: WindowDescriptor {
        //         width: settings.width as f32,
        //         height: settings.height as f32,
        //         position: WindowPosition::At(Vec2::new(
        //             settings.pos_x as f32,
        //             settings.pos_y as f32,
        //         )),
        //         title: format!("window {}: {}", i, name),
        //         ..default()
        //     },
        // });

        // second window camera
        let entity = commands
            .spawn(Camera3dBundle {
                transform: settings.orientation.get_transform(),
                camera: Camera {
                    // target: RenderTarget::Window(window_id),
                    target: RenderTarget::Image(offscreen_image.clone()),
                    priority: -1,
                    ..default()
                },
                projection: Projection::Orthographic(OrthographicProjection {
                    scaling_mode: ScalingMode::FixedHorizontal(10.0),
                    ..default()
                }),
                ..default()
            })
            .insert(*render_layer)
            .id();

        *window2d = Some(resources::EditorWindow2d {
            camera: entity,
            window_id,
            settings,
        });
    }

    // extract name and Some(Window2d) values into name -> Window2d map
    editor_windows_2d.windows = transforms
        .drain(..)
        .filter_map(|(name, window2d, _, _, _)| {
            window2d.map(|window2d| (name.to_owned(), window2d))
        })
        .collect();

    editor_windows_2d.view_max = Vec3::splat(f32::INFINITY);
    editor_windows_2d.view_min = Vec3::splat(f32::NEG_INFINITY);
}

pub fn track_window_props(
    mut editor_windows_2d: ResMut<resources::EditorWindows2d>,

    mut resize_events: EventReader<WindowResized>,
    mut move_events: EventReader<WindowMoved>,
) {
    for event in resize_events.iter() {
        for (name, window2d) in &mut editor_windows_2d.windows {
            if event.id == window2d.window_id {
                info!("{} resize: {} {}", name, event.width, event.height);
                window2d.settings.width = event.width as i32;
                window2d.settings.height = event.height as i32;
            }
        }
    }
    for event in move_events.iter() {
        for (name, window2d) in &mut editor_windows_2d.windows {
            if event.id == window2d.window_id {
                info!("{} move: {} {}", name, event.position.x, event.position.y);
                window2d.settings.pos_x = event.position.x;
                window2d.settings.pos_y = event.position.y;
            }
        }
    }
}

pub fn write_window_settings(
    mut last_written_settings: Local<BTreeMap<String, EditorWindowSettings>>,
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

pub fn track_focused_window(
    mut editor_windows_2d: ResMut<resources::EditorWindows2d>,
    mut focus_events: EventReader<WindowFocused>,
    mut cursor_moved: EventReader<CursorMoved>,
) {
    let mut editor_windows_2d = &mut *editor_windows_2d;
    let mut new_focus = None;
    let mut focus_lost = false;
    for event in focus_events.iter() {
        for (name, window) in &editor_windows_2d.windows {
            if event.focused && window.window_id == event.id {
                new_focus = Some((name.clone(), event.id));
            } else if !event.focused && window.window_id == event.id {
                focus_lost = true
            }
        }
    }

    if new_focus.is_some() {
        editor_windows_2d.focused = new_focus;
        info!("focus changed: {:?}", editor_windows_2d.focused);
    } else if focus_lost {
        editor_windows_2d.focused = None;
        info!("focus lost");
    }

    if editor_windows_2d.focused.is_some() {
        for event in cursor_moved.iter() {
            editor_windows_2d.cursor_pos = event.position;
        }
    }
}

pub fn control_input_system(
    keycodes: Res<Input<KeyCode>>,
    mouse_buttons: Res<Input<MouseButton>>,
    mut editor_windows_2d: ResMut<resources::EditorWindows2d>,
    mut mouse_wheel: EventReader<MouseWheel>,
    camera_query: Query<(&GlobalTransform, &Camera)>,
    mut transform_query: Query<&mut Transform>,
    mut projection_query: Query<&mut Projection>,
) {
    let editor_windows_2d = &mut *editor_windows_2d;

    let Some((focus_name, _focus_id)) = &editor_windows_2d.focused else {return};

    for event in mouse_wheel.iter() {
        let scroll_step = if keycodes.pressed(KeyCode::LAlt) {
            5.0
        } else {
            2.0
        };
        let dir = -event.y.signum() * scroll_step; // scroll down -> zooms out

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

            if *scaling + dir > 0.0 {
                *scaling += dir;
            }
        }
    }

    'block: {
        let Some((focused_name, _)) = &editor_windows_2d.focused else { break 'block;};
        let Some(window) = editor_windows_2d.windows.get(focused_name) else { break 'block; };
        let Ok((global_transform, camera)) = camera_query.get(window.camera) else {
            warn!("2d window camera not found: {:?}", window.camera); 
            break 'block;
        };

        if mouse_buttons.just_pressed(MouseButton::Middle) {
            info!("middle down");
            let Some(ray) = camera.viewport_to_world(global_transform, editor_windows_2d.cursor_pos) else {
                warn!("viewport_to_world failed in {}", focused_name); 
                break 'block;
            };
            let mut transforms = Vec::new();
            for (_name, window) in &editor_windows_2d.windows {
                if let Ok(transform) = transform_query.get(window.camera) {
                    transforms.push((window.camera, *transform));
                }
            }

            editor_windows_2d.translate_drag = Some(TranslateDrag {
                start_ray: ray,
                start_focus: focus_name.clone(),
                start_global_transform: *global_transform,
                start_transforms: transforms,
            });
        } else if mouse_buttons.just_released(MouseButton::Middle) {
            info!("middle up");
            editor_windows_2d.translate_drag = None;
        } else if let Some(TranslateDrag {
            start_ray,
            start_focus: _,
            start_global_transform,
            start_transforms,
        }) = &editor_windows_2d.translate_drag
        {
            let Some(ray) = camera.viewport_to_world(start_global_transform, editor_windows_2d.cursor_pos) else {
                warn!("viewport_to_world failed in {}", focused_name); 
                break 'block;
            };
            let d = start_ray.origin - ray.origin;
            for (entity, start_transform) in start_transforms {
                if let Ok(mut transform) = transform_query.get_mut(*entity) {
                    transform.translation = start_transform.translation + d;
                }
            }
        }
    }
    if keycodes.just_pressed(KeyCode::F2) {
        for (_, mut window) in &mut editor_windows_2d.windows {
            let Ok(mut transform) = transform_query.get_mut(window.camera) else {
                warn!("2d window camera transform / projection not found: {:?}", window.camera); 
                continue;
            };

            window.settings.orientation = window.settings.orientation.flipped();
            *transform = window.settings.orientation.get_transform();
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
                button: WmMouseButton::Middle,
                pointer_state,
            } => {
                let Some(window) = editor_windows_2d.windows.get(focused_name) else { continue; };
                let Ok((global_transform, camera)) = camera_query.get(window.camera) else {
                    warn!("2d window camera not found: {:?}", window.camera); 
                    continue;
                };
                info!("middle down");
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

                editor_windows_2d.translate_drag = Some(TranslateDrag {
                    start_ray: ray,
                    start_focus: focused_name.to_string(),
                    start_global_transform: *global_transform,
                    start_transforms: transforms,
                });
            }
            util::WmEvent::DragUpdate {
                window: focused_name,
                button: WmMouseButton::Middle,
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
                if let Some(TranslateDrag {
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
                    let mut d = start_ray.origin - ray.origin;
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
                button: WmMouseButton::Middle,
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
    selection: Res<Selection>,
    keycodes: Res<Input<KeyCode>>,
    mut mouse_button: EventReader<MouseButtonInput>,
    editor_windows_2d: Res<resources::EditorWindows2d>,

    camera_query: Query<(&GlobalTransform, &Camera)>,
    brush_query: Query<&EditorObject, Without<DragAction>>,
    mut active_drag_query: Query<(Entity, &DragAction, &mut EditorObject), With<DragAction>>,

    mut csg_repr_query: Query<&mut components::CsgRepresentation, With<DragAction>>,
    mut transform_query: Query<&mut Transform>,
) {
    let Some((focus_name, _focus_id)) = &editor_windows_2d.focused else {return};

    // LControl is 'select mode'. Prohibits start of edit actions (but they can still update or end)
    let start_edit_allowed = !keycodes.pressed(KeyCode::LControl);

    'block: {
        let Some((focused_name, _)) = &editor_windows_2d.focused else { break 'block;};
        let Some(window) = editor_windows_2d.windows.get(focused_name) else { break 'block; };
        let Ok((global_transform, camera)) = camera_query.get(window.camera) else {
            warn!("2d window camera not found: {:?}", window.camera); 
            break 'block;
        };

        let Some(ray) = camera.viewport_to_world(global_transform, editor_windows_2d.cursor_pos) else {
            warn!("viewport_to_world failed in {}", focused_name); 
            break 'block;
        };

        for event in mouse_button.iter() {
            if event.button == MouseButton::Left
                && event.state == ButtonState::Pressed
                && start_edit_allowed
            {
                info!("click ray {}: {:?}", focus_name, ray);

                if let Some(primary) = selection.primary {
                    match brush_query.get(primary) {
                        Ok(EditorObject::Brush(brush)) => {
                            let affected_faces = brush.get_planes_behind_ray(ray);

                            if !affected_faces.is_empty() {
                                commands.entity(primary).insert(DragAction {
                                    start_ray: ray,
                                    action: DragActionType::Face { affected_faces },
                                });
                                info!("start face drag for {:?}", primary); // the crowd put on their affected_faces as The Iron Sheik did his signature face-drag on el Pollo Loco
                            } else {
                                let affected_faces = brush
                                    .planes
                                    .iter()
                                    .enumerate()
                                    .map(|(i, face)| (i, face.w))
                                    .collect();
                                commands.entity(primary).insert(DragAction {
                                    start_ray: ray,
                                    action: DragActionType::WholeBrush { affected_faces },
                                });
                                info!("start whole-brush drag for {:?}", primary);
                            }
                        }
                        Ok(EditorObject::PointLight(_)) => {
                            if let Ok(transform) = transform_query.get(primary) {
                                info!("light drag start");

                                commands.entity(primary).insert(DragAction {
                                    start_ray: ray,
                                    action: DragActionType::NonBrush {
                                        start_translation: transform.translation,
                                    },
                                });
                            }
                        }
                        _ => (),
                    }
                }
            } else if event.button == MouseButton::Left && event.state == ButtonState::Released {
                for (entity, _, _) in &active_drag_query {
                    commands.entity(entity).remove::<DragAction>();
                    info!("stop drag for {:?}", entity);
                }
            }
        }

        // update dragged objects. Do this in two steps, only touch EditorObject as mutable if there is a relevant change
        // to prevent triggering the bevy change detection.
        let mut csg_updates = Vec::new();
        let mut transform_updates = Vec::new();
        for (entity, drag_action, editor_object) in &active_drag_query {
            let drag_delta = ray.origin - drag_action.start_ray.origin;

            debug!("drag: {:?} on {:?}", drag_delta, entity);

            match (&drag_action.action, editor_object) {
                (DragActionType::Face { affected_faces }, EditorObject::Brush(brush))
                | (DragActionType::WholeBrush { affected_faces }, EditorObject::Brush(brush)) /* yay, free implementation */ => {
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
                                        EditorObject::Brush(new_brush),
                                        CsgRepresentation {
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
                (DragActionType::NonBrush{ start_translation }, EditorObject::PointLight(_)) => {
                    transform_updates.push((entity, *start_translation + drag_delta));
                },
                _ => warn!( "invalid combinaton of editor object and drag action."),
            }
        }

        // info!("updates: {:?}", updates);

        for (entity, (obj, bounds)) in csg_updates {
            info!("apply update on {:?}", entity);
            if let Ok((_, _, mut target_obj)) = active_drag_query.get_mut(entity) {
                if let Ok(mut target_bounds) = csg_repr_query.get_mut(entity) {
                    *target_obj = obj;
                    *target_bounds = bounds;
                }
            }
        }

        for (entity, translation) in transform_updates {
            if let Ok(mut transform) = transform_query.get_mut(entity) {
                transform.translation = translation;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn edit_input_wm_system(
    mut commands: Commands,
    mut event_reader: EventReader<util::WmEvent>,
    selection: Res<Selection>,
    keycodes: Res<Input<KeyCode>>,
    editor_windows_2d: Res<resources::EditorWindows2d>,

    camera_query: Query<(&GlobalTransform, &Camera)>,
    brush_query: Query<&EditorObject, Without<DragAction>>,
    mut active_drag_query: Query<(Entity, &DragAction, &mut EditorObject), With<DragAction>>,

    mut csg_repr_query: Query<&mut components::CsgRepresentation, With<DragAction>>,
    mut transform_query: Query<&mut Transform>,
) {
    for event in event_reader.iter() {
        info!("event edit: {:?}", event);
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
                    match brush_query.get(primary) {
                        Ok(EditorObject::Brush(brush)) => {
                            let affected_faces = brush.get_planes_behind_ray(ray);

                            if !affected_faces.is_empty() {
                                commands.entity(primary).insert(DragAction {
                                    start_ray: ray,
                                    action: DragActionType::Face { affected_faces },
                                });
                                info!("start face drag for {:?}", primary); // the crowd put on their affected_faces as The Iron Sheik did his signature face-drag on el Pollo Loco
                            } else {
                                let affected_faces = brush
                                    .planes
                                    .iter()
                                    .enumerate()
                                    .map(|(i, face)| (i, face.w))
                                    .collect();
                                commands.entity(primary).insert(DragAction {
                                    start_ray: ray,
                                    action: DragActionType::WholeBrush { affected_faces },
                                });
                                info!("start whole-brush drag for {:?}", primary);
                            }
                        }
                        Ok(EditorObject::PointLight(_)) => {
                            if let Ok(transform) = transform_query.get(primary) {
                                info!("light drag start");

                                commands.entity(primary).insert(DragAction {
                                    start_ray: ray,
                                    action: DragActionType::NonBrush {
                                        start_translation: transform.translation,
                                    },
                                });
                            }
                        }
                        _ => (),
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
                info!("left down");
                let Some(ray) = camera.viewport_to_world(global_transform, pointer_state.get_pos_origin_down()) else {
                    warn!("viewport_to_world failed in {}", focused_name); 
                    continue;
                };

                // update dragged objects. Do this in two steps, only touch EditorObject as mutable if there is a relevant change
                // to prevent triggering the bevy change detection.
                let mut csg_updates = Vec::new();
                let mut transform_updates = Vec::new();
                for (entity, drag_action, editor_object) in &active_drag_query {
                    let drag_delta = ray.origin - drag_action.start_ray.origin;

                    debug!("drag: {:?} on {:?}", drag_delta, entity);

                    match (&drag_action.action, editor_object) {
                        (DragActionType::Face { affected_faces }, EditorObject::Brush(brush))
                        | (DragActionType::WholeBrush { affected_faces }, EditorObject::Brush(brush)) /* yay, free implementation */ => {
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
                                                EditorObject::Brush(new_brush),
                                                CsgRepresentation {
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
                        (DragActionType::NonBrush{ start_translation }, EditorObject::PointLight(_)) => {
                            transform_updates.push((entity, *start_translation + drag_delta));
                        },
                        _ => warn!( "invalid combinaton of editor object and drag action."),
                    }
                }
                // info!("updates: {:?}", updates);

                for (entity, (obj, bounds)) in csg_updates {
                    info!("apply update on {:?}", entity);
                    if let Ok((_, _, mut target_obj)) = active_drag_query.get_mut(entity) {
                        if let Ok(mut target_bounds) = csg_repr_query.get_mut(entity) {
                            *target_obj = obj;
                            *target_bounds = bounds;
                        }
                    }
                }

                for (entity, translation) in transform_updates {
                    if let Ok(mut transform) = transform_query.get_mut(entity) {
                        transform.translation = translation;
                    }
                }
            }

            util::WmEvent::DragEnd {
                window,
                button: util::WmMouseButton::Left,
                pointer_state,
            } => {
                for (entity, _, _) in &active_drag_query {
                    commands.entity(entity).remove::<DragAction>();
                    info!("stop drag for {:?}", entity);
                }
            }
            _ => (),
        }
    }
}

#[derive(Resource)]
pub struct ClickTimer {
    pub timer: Timer,
}

impl Default for ClickTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.5, TimerMode::Once),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn select_input_system(
    time: Res<Time>,
    mut click_timer: Local<ClickTimer>,
    mouse_buttons: Res<Input<MouseButton>>,
    keycodes: Res<Input<KeyCode>>,
    mut selection: ResMut<Selection>,
    editor_windows_2d: Res<resources::EditorWindows2d>,
    camera_query: Query<(&GlobalTransform, &Camera)>,
    brush_query: Query<(Entity, &EditorObject, &CsgRepresentation)>,
) {
    click_timer.timer.tick(time.delta());
    let Some((focused_name, _)) = &editor_windows_2d.focused else { return };

    if mouse_buttons.just_pressed(MouseButton::Left) {
        click_timer.timer.reset();
    } else if (mouse_buttons.just_released(MouseButton::Left))
        && keycodes.pressed(KeyCode::LControl)
        && !click_timer.timer.finished()
    {
        info!("select");

        'block: {
            let Some(window) = editor_windows_2d.windows.get(focused_name) else { break 'block; };
            let Ok((global_transform, camera)) = camera_query.get(window.camera) else {
                warn!("2d window camera not found: {:?}", window.camera);
                break 'block;
            };

            let Some(ray) = camera.viewport_to_world(global_transform, editor_windows_2d.cursor_pos) else {
                warn!("viewport_to_world failed in {}", focused_name); 
                break 'block;
            };

            info!(
                "minmax: {:?} {:?}",
                editor_windows_2d.view_min, editor_windows_2d.view_max
            );
            // editor_windows_2d.
            let selection_set = brush_query
                .iter()
                .filter_map(|(entity, obj, csg)| match obj {
                    EditorObject::Brush(brush) => {
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
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
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

#[allow(clippy::too_many_arguments)]
pub fn select_input_wm_system(
    time: Res<Time>,
    mut event_reader: EventReader<util::WmEvent>,
    mut click_timer: Local<ClickTimer>,
    mouse_buttons: Res<Input<MouseButton>>,
    keycodes: Res<Input<KeyCode>>,
    mut selection: ResMut<Selection>,
    editor_windows_2d: Res<resources::EditorWindows2d>,
    camera_query: Query<(&GlobalTransform, &Camera)>,
    brush_query: Query<(Entity, &EditorObject, &CsgRepresentation)>,
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
            let selection_set = brush_query
                .iter()
                .filter_map(|(entity, obj, csg)| match obj {
                    EditorObject::Brush(brush) => {
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
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
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
