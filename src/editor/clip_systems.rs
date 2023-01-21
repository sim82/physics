use crate::editor::{edit_commands, util::SnapToGrid};

use super::{components, edit_commands::EditCommands, resources, util};
use bevy::{prelude::*, render::mesh};
use shared::render_layers;

pub fn clip_plane_setup_system(
    mut commands: Commands,
    materials_res: Res<resources::Materials>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // let mesh = mesh::shape::Plane { size: 10.0 }.into();
    let mesh = mesh::shape::Box::new(10.0, 10.0, 0.1).into();

    commands
        .spawn(components::ClipPlaneBundle::default())
        .add_children(|commands| {
            commands.spawn((
                PbrBundle {
                    mesh: meshes.add(mesh),
                    material: materials_res.brush_2d.clone(),

                    ..default()
                },
                render_layers::ortho_views(),
            ));
        });
}

#[derive(Default)]
pub enum NextClipPoint {
    #[default]
    Point0,
    Point1,
}

pub fn clip_plane_control_system(
    mut event_reader: EventReader<util::WmEvent>,
    mut clip_plane_query: Query<&mut components::ClipPlane>,
    editor_windows_2d: Res<resources::EditorWindows2d>,
    camera_query: Query<(&GlobalTransform, &Camera)>,
    mut next_clip_point: Local<NextClipPoint>,
) {
    for event in event_reader.iter() {
        if let util::WmEvent::Clicked {
            window: focused_name,
            button: util::WmMouseButton::Left,
            pointer_state,
        } = *event
        {
            if !pointer_state.modifiers.alt {
                continue;
            }

            info!("event: {:?}", event);
            let Some(window) = editor_windows_2d.windows.get(focused_name) else { continue };
            let Ok((global_transform, camera)) = camera_query.get(window.camera) else {
                warn!("2d window camera not found: {:?}", window.camera);
                continue;
            };

            let Some(ray) = camera.viewport_to_world(global_transform, pointer_state.get_pos_origin_down()) else {
                warn!("viewport_to_world failed in {}", focused_name);
                continue;
            };

            let Ok(mut clip_plane) = clip_plane_query.get_single_mut() else {
                continue;
            };

            const SNAP: f32 = 0.5;
            match *next_clip_point {
                NextClipPoint::Point0 => {
                    info!("set clip point 0");
                    clip_plane.points[0] = window
                        .orientation
                        .mix(ray.origin, clip_plane.points[0])
                        .snap(SNAP);
                    // clip_plane.points[0] = ray.origin;
                    *next_clip_point = NextClipPoint::Point1;
                }
                NextClipPoint::Point1 => {
                    info!("set clip point 1 & 2");

                    clip_plane.points[1] = window
                        .orientation
                        .mix(ray.origin, clip_plane.points[1])
                        .snap(SNAP);
                    clip_plane.points[2] =
                        (window.orientation.mix(ray.origin, clip_plane.points[2]) + ray.direction)
                            .snap(SNAP);

                    *next_clip_point = NextClipPoint::Point0;
                }
            }
        }
    }
}

pub fn clip_plane_vis_system(
    mut clip_plane_changed_query: Query<
        (Entity, &mut Transform, &components::ClipPlane),
        Changed<components::ClipPlane>,
    >,
    // mut clip_plane_vis_query: Query<&mut Transform>,
) {
    for (_entity, mut transform, clip_plane) in &mut clip_plane_changed_query {
        info!("clip plane changed: {:?}", clip_plane.points);
        let plane = clip_plane.get_plane();
        *transform = Transform::from_translation(clip_plane.points[0]);
        transform.look_at(
            clip_plane.points[0] + plane.normal,
            (clip_plane.points[2] - clip_plane.points[1]).normalize_or_zero(),
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub fn clip_preview_system(
    mut commands: Commands,
    keycodes: Res<Input<KeyCode>>,
    mut edit_commands: EditCommands,
    materials_res: Res<resources::Materials>,
    material_browser: Res<resources::MaterialBrowser>,
    mut clip_state: ResMut<resources::ClipState>,
    selected_query: Query<(Entity, &Children), With<components::Selected>>,
    brush_changed_query: Query<(), (With<components::Selected>, Changed<csg::Brush>)>,
    despawn_query: Query<Entity, With<components::ClipPreview>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut vis_query: Query<&mut Visibility, Without<components::ClipPreview>>,
    mut clip_vis_query: Query<
        &mut Visibility,
        (With<components::ClipPreview>, Without<components::Selected>),
    >,
    clip_plane_query: Query<&components::ClipPlane>,
    clip_plane_changed_query: Query<(), Changed<components::ClipPlane>>,
) {
    if brush_changed_query.is_empty()
        && clip_plane_changed_query.is_empty()
        && clip_state.clip_mode == clip_state.last_clip_mode
        && !keycodes.just_pressed(KeyCode::R)
        && !keycodes.just_pressed(KeyCode::G)
    {
        return;
    }

    let Ok((selected_entity, children)) = selected_query.get_single() else {
        return;
    };

    let Ok((material_props, brush)) = edit_commands.brush_query.get(selected_entity) else {
        return;
    };

    let Ok(clip_plane) = clip_plane_query.get_single().map(|clip_plane| clip_plane.get_plane()) else {
        return;
    };

    if clip_state.clip_mode && !clip_state.last_clip_mode {
        info!("to clip mode");
        for mut vis in &mut clip_vis_query {
            vis.is_visible = true;
        }
        for entity in children {
            if let Ok(mut vis) = vis_query.get_mut(*entity) {
                vis.is_visible = false;
            }
        }
        clip_state.last_clip_mode = clip_state.clip_mode;
    } else if !clip_state.clip_mode && clip_state.last_clip_mode {
        info!("from clip mode");

        for mut vis in &mut clip_vis_query {
            vis.is_visible = false;
        }
        for entity in children {
            if let Ok(mut vis) = vis_query.get_mut(*entity) {
                vis.is_visible = true;
            }
        }
        // clip_points.despawn();
        clip_state.next_point = 0;
        clip_state.last_clip_mode = clip_state.clip_mode;
    }

    for entity in &despawn_query {
        commands.entity(entity).despawn_recursive();
    }

    if !clip_state.clip_mode {
        return;
    }

    // let plane = csg::Plane::from_points_slice(&clip_state.plane_points);
    info!("plane: {:?} {:?}", clip_state.plane_points, clip_plane);
    let clipped1 = clipped_brush(
        brush.clone(),
        clip_plane,
        material_props,
        &material_browser.selected_material,
    );
    let clipped2 = clipped_brush(
        brush.clone(),
        clip_plane.flipped(),
        material_props,
        &material_browser.selected_material,
    );

    // info!("res: {:?}", res);
    let brushes = [
        (0, &clipped1, materials_res.brush_clip_red.clone()),
        (1, &clipped2, materials_res.brush_clip_green.clone()),
    ];

    for (i, clipped, material) in brushes {
        let Some((brush, _)) = clipped else { continue };
        let csg: Result<csg::Csg, _> = brush.clone().try_into();
        if let Ok(csg) = csg {
            let (mesh, origin) = (&csg).into();

            let transform = Transform::from_translation(origin);
            commands.spawn((
                PbrBundle {
                    mesh: meshes.add(mesh),
                    material,
                    transform,
                    visibility: Visibility {
                        is_visible: clip_state.clip_mode,
                    },
                    ..default()
                },
                render_layers::ortho_views(),
                components::ClipPreview,
            ));
        } else {
            info!("clip failed {}", i);
        }
    }

    if keycodes.just_pressed(KeyCode::R) {
        info!("use red: {:?} -> {:?}", brush, clipped1);
        // let mut new_material_props = material_props.clone();
        if let Some((brush, material_props)) = clipped1 {
            let res = edit_commands.apply(edit_commands::clip_brush::Command {
                entity: selected_entity,
                start_brush: brush.clone(),
                start_material_props: material_props.clone(),
                brush,
                material_props,
            });
            if let Err(err) = res {
                warn!("failed to update brush after clip: {:?}", err);
            }
        }

        clip_state.clip_mode = false;
    } else if keycodes.just_pressed(KeyCode::G) {
        info!("use green: {:?} -> {:?}", brush, clipped2);
        // let mut new_material_props = material_props.clone();
        if let Some((brush, material_props)) = clipped2 {
            let res = edit_commands.apply(edit_commands::clip_brush::Command {
                entity: selected_entity,
                start_brush: brush.clone(),
                start_material_props: material_props.clone(),
                brush,
                material_props,
            });
            if let Err(err) = res {
                warn!("failed to update brush after clip: {:?}", err);
            }
        }

        clip_state.clip_mode = false;
    }
}

fn clipped_brush(
    mut brush: csg::Brush,
    clip_plane: csg::Plane,
    material_props: &components::BrushMaterialProperties,
    material_name: &str,
) -> Option<(csg::Brush, components::BrushMaterialProperties)> {
    let res = brush.add_plane(clip_plane);
    if !res {
        info!("brush clip failed");
        return None;
    }

    let remap = brush.remove_degenerated();
    let materials = remap
        .iter()
        .map(|old| {
            material_props
                .materials
                .get(*old as usize)
                .cloned()
                .unwrap_or_else(|| material_name.to_string())
        })
        .collect();
    Some((brush, components::BrushMaterialProperties { materials }))
}
