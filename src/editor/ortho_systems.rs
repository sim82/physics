use std::collections::BTreeMap;

use bevy::{
    input::{
        mouse::{MouseButtonInput, MouseWheel},
        ButtonState,
    },
    prelude::*,
    render::camera::{Projection, RenderTarget, ScalingMode},
    utils::HashMap,
    window::{CreateWindow, WindowFocused, WindowId, WindowResized},
};

use super::{
    components::EditorObject,
    resources::{self, EditorWindowSettings, Selection, LOWER_WINDOW, UPPER_WINDOW},
    util::Orientation2d,
};
use crate::{
    csg::{self},
    editor::{components::BrushDragAction, util::HackViewportToWorld},
};
// systems related to 2d windows

pub fn setup_editor_window(
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
            // Transform::from_xyz(0.0, 6.0, 0.0).looking_at(Vec3::ZERO, Vec3::X),
        ),
        (
            LOWER_WINDOW,
            None,
            Orientation2d::Front,
            // Transform::from_xyz(-6.0, 0.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
        ),
    ];
    for (i, (name, window2d, t)) in transforms.iter_mut().enumerate() {
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
        create_window_events.send(CreateWindow {
            id: window_id,
            descriptor: WindowDescriptor {
                width: settings.width as f32,
                height: settings.height as f32,
                position: WindowPosition::At(Vec2::new(
                    settings.pos_x as f32,
                    settings.pos_y as f32,
                )),
                title: format!("window {}: {}", i, name),
                ..default()
            },
        });

        // second window camera
        let entity = commands
            .spawn_bundle(Camera3dBundle {
                transform: settings.orientation.get_transform(),
                camera: Camera {
                    target: RenderTarget::Window(window_id),
                    ..default()
                },
                projection: Projection::Orthographic(OrthographicProjection {
                    scaling_mode: ScalingMode::FixedHorizontal(10.0),
                    ..default()
                }),
                ..default()
            })
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
        .filter_map(|(name, window2d, _)| window2d.map(|window2d| (name.to_owned(), window2d)))
        .collect()
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

#[allow(clippy::too_many_arguments)]
pub fn editor_windows_2d_input_system(
    mut commands: Commands,
    selection: Res<Selection>,
    keycodes: Res<Input<KeyCode>>,
    mut mouse_button: EventReader<MouseButtonInput>,
    mut mouse_wheel: EventReader<MouseWheel>,
    // mut mouse_wheel: EventReader<Mouse>,
    mut editor_windows_2d: ResMut<resources::EditorWindows2d>,

    mut camera_query: Query<(
        &mut Transform,
        &GlobalTransform,
        &mut Projection,
        &mut Camera,
    )>,
    brush_query: Query<&EditorObject, Without<BrushDragAction>>,
    mut active_drag_query: Query<
        (Entity, &BrushDragAction, &mut EditorObject),
        With<BrushDragAction>,
    >,
) {
    let Some((focus_name, _focus_id)) = &editor_windows_2d.focused else {return};

    for event in mouse_wheel.iter() {
        let dir = event.y.signum();

        // for mut transform in &mut camera_query {
        //     todo!()
        // }

        for (_name, window) in &editor_windows_2d.windows {
            let Ok((_transform, _, mut projection, _camera)) = camera_query.get_mut(window.camera) else {
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

            *scaling += dir;
        }
    }

    // meh... seems as if I'm up to something
    #[allow(clippy::never_loop)]
    'outer: loop {
        let Some((focused_name, _)) = &editor_windows_2d.focused else { break 'outer;};
        let Some(window) = editor_windows_2d.windows.get(focused_name) else { break 'outer; };
        let Ok((_, global_transform, _, camera)) = camera_query.get_mut(window.camera) else {
            warn!("2d window camera not found: {:?}", window.camera); 
            break 'outer;
        };

        let Some(ray) = camera.viewport_to_world(global_transform, editor_windows_2d.cursor_pos) else {
            warn!("viewport_to_world failed in {}", focused_name); 
            break 'outer;
        };

        for event in mouse_button.iter() {
            if event.button == MouseButton::Left && event.state == ButtonState::Pressed {
                info!("click ray {}: {:?}", focus_name, ray);
                info!("start drag");

                if let Some(primary) = selection.primary {
                    if let Ok(EditorObject::Brush(brush)) = brush_query.get(primary) {
                        let affected_faces = brush.get_planes_behind_ray(ray);

                        commands.entity(primary).insert(BrushDragAction {
                            start_ray: ray,
                            affected_faces,
                        });
                    }
                }
            } else if event.button == MouseButton::Left && event.state == ButtonState::Released {
                for (entity, _, _) in &active_drag_query {
                    commands.entity(entity).remove::<BrushDragAction>();
                }
            }
        }

        for (entity, drag_action, mut editor_object) in &mut active_drag_query {
            let EditorObject::Brush(brush) = &mut *editor_object else {
                warn!( "drag: not a brush: {:?}", entity);
                continue;
            };

            let drag_delta = ray.origin - drag_action.start_ray.origin;

            info!("drag: {:?}", drag_delta);

            for (face, start_w) in &drag_action.affected_faces {
                let normal = brush.planes[*face].normal;
                let d = drag_delta.dot(normal);

                let mut new_brush = brush.clone();
                new_brush.planes[*face].w = *start_w + d;

                // apply only if target is not degenerated
                if std::convert::TryInto::<csg::Csg>::try_into(new_brush.clone()).is_ok() {
                    *brush = new_brush
                }
            }
        }

        break;
    }

    if keycodes.just_pressed(KeyCode::F2) {
        for (_, mut window) in &mut editor_windows_2d.windows {
            let Ok((mut transform, _, _, _)) = camera_query.get_mut(window.camera) else {
                warn!("2d window camera transform / projection not found: {:?}", window.camera); 
                continue;
            };

            window.settings.orientation = window.settings.orientation.flipped();

            *transform = window.settings.orientation.get_transform();
        }
    }
}
