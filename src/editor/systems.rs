use std::{collections::BTreeMap, str::FromStr};

use bevy::{
    input::mouse::MouseWheel,
    pbr::wireframe::Wireframe,
    prelude::{shape::Cube, *},
    render::{
        camera::{Projection, RenderTarget, ScalingMode},
        primitives::Aabb,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
    utils::{HashMap, Instant, Uuid},
    window::{CreateWindow, WindowId, WindowResized},
};

use super::{
    components::{CsgOutput, EditorObject, SelectionVis},
    resources::{self, EditorWindowSettings, Selection, LOWER_WINDOW, UPPER_WINDOW},
};
use crate::{
    csg::{self},
    editor::util::add_csg,
};

#[allow(clippy::too_many_arguments)]
pub fn editor_input_system(
    mut commands: Commands,

    mut offset: Local<Option<Vec3>>,

    keycodes: Res<Input<KeyCode>>,
    mut mouse_wheel: EventReader<MouseWheel>,

    mut selection: ResMut<Selection>,

    mut query: Query<&mut EditorObject>,
) {
    if keycodes.just_pressed(KeyCode::K) {
        let entity = commands
            .spawn()
            .insert(EditorObject::MinMax(Vec3::splat(-1.0), Vec3::splat(1.0)))
            .id();

        selection.primary = Some(entity);
    }

    if keycodes.just_pressed(KeyCode::B) {
        let entity = commands
            .spawn()
            .insert(EditorObject::Brush(csg::Brush::default()))
            .id();

        selection.primary = Some(entity);
    }
    if keycodes.just_pressed(KeyCode::L) {
        let entity = commands
            .spawn()
            .insert(EditorObject::Csg(
                csg::Cube::new(Vec3::splat(2.0), 0.5).into(),
            ))
            .id();

        selection.primary = Some(entity);
    }
    if keycodes.just_pressed(KeyCode::M) {
        if let Some(selected_entity) = selection.primary {
            if let Ok(_brush) = query.get_mut(selected_entity) {
                info!("spawn");
                let offset = offset.get_or_insert(Vec3::splat(2.5));

                let entity = commands
                    .spawn()
                    // .insert(Brush::Csg(Cube::new(*offset, 0.5).into()))
                    .insert(EditorObject::Csg(
                        csg::Cylinder {
                            start: Vec3::new(0.0, -1.0, 0.0) + *offset,
                            end: Vec3::new(0.0, 1.0, 0.0) + *offset,
                            radius: 2.0,
                            ..default()
                        }
                        // csg::Sphere::new(*offset, 1.0, 16, 8)
                        .into(),
                    ))
                    .id();

                *offset += Vec3::splat(0.5);
                selection.primary = Some(entity);
            }
        }
    }
    if keycodes.just_pressed(KeyCode::N) {
        if let Some(selection) = selection.primary {
            if let Ok(mut brush) = query.get_mut(selection) {
                if let EditorObject::Csg(ref mut csg) = *brush {
                    csg.invert();
                }
            }
        }
    }

    let mut dmin = Vec3::ZERO;
    let mut dmax = Vec3::ZERO;

    for event in mouse_wheel.iter() {
        let d = event.y.signum() * 0.1;

        if keycodes.pressed(KeyCode::Q) {
            dmin.x -= d;
            dmax.x += d;
        }
        if keycodes.pressed(KeyCode::A) {
            dmin.y -= d;
            dmax.y += d;
        }
        if keycodes.pressed(KeyCode::Z) {
            dmin.z -= d;
            dmax.z += d;
        }
        if keycodes.pressed(KeyCode::W) {
            dmin.x += d;
            dmax.x += d;
        }
        if keycodes.pressed(KeyCode::S) {
            dmin.y += d;
            dmax.y += d;
        }
        if keycodes.pressed(KeyCode::X) {
            dmin.z += d;
            dmax.z += d;
        }
    }

    if let Some(selection) = selection.primary {
        if let Ok(mut brush) = query.get_mut(selection) {
            if dmin.length() > 0.0 || dmax.length() > 0.0 {
                match *brush {
                    EditorObject::MinMax(ref mut min, ref mut max) => {
                        *min += dmin;
                        *max += dmax;
                    }
                    EditorObject::Csg(ref mut csg) => {
                        csg.translate(dmin);
                    }
                    EditorObject::Brush(ref mut brush) => {
                        let mut new_brush = brush.clone();
                        new_brush.planes[0].w += dmax.x;
                        new_brush.planes[1].w -= dmin.x;
                        new_brush.planes[2].w += dmax.y;
                        new_brush.planes[3].w -= dmin.y;
                        new_brush.planes[4].w += dmax.z;
                        new_brush.planes[5].w -= dmin.z;
                        if std::convert::TryInto::<csg::Csg>::try_into(new_brush.clone()).is_ok() {
                            *brush = new_brush
                        }
                    }
                }
            }
        }
    }

    // if mouse.any_pressed(MouseButton::Other(()))

    // if keycodes.just_pr
}

pub fn update_brushes_system(
    mut commands: Commands,

    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,

    mut query: Query<(Entity, &EditorObject), Changed<EditorObject>>,
    query_cleanup: Query<(&Handle<Mesh>, &Handle<StandardMaterial>)>,
) {
    // for (entity, brush) in &mut query {
    //     // let entity = spawn_box(
    //     //     &mut commands,
    //     //     material,
    //     //     &mut meshes,
    //     //     Vec3::splat(-1.0),
    //     //     Vec3::splat(1.0),
    //     // );
    //     if let Ok((mesh, material)) = query_cleanup.get(entity) {
    //         info!("cleanup {:?} {:?}", mesh, material);
    //         meshes.remove(mesh);
    //         if let Some(material) = materials.remove(material) {
    //             if let Some(image) = material.base_color_texture {
    //                 info!("cleanup {:?}", image);
    //                 images.remove(image);
    //             }
    //         }
    //     }
    //     match brush {
    //         EditorObject::MinMax(min, max) => {
    //             let uv_test = images.add(Image::new(
    //                 Extent3d {
    //                     width: test_texture::TW as u32,
    //                     height: test_texture::TH as u32,
    //                     depth_or_array_layers: 1,
    //                 },
    //                 TextureDimension::D2,
    //                 test_texture::create(),
    //                 TextureFormat::Rgba8Unorm,
    //             ));

    //             let material = materials.add(StandardMaterial {
    //                 base_color_texture: Some(uv_test),
    //                 metallic: 0.9,
    //                 perceptual_roughness: 0.1,
    //                 ..Default::default()
    //             });

    //             add_box(&mut commands, entity, material, &mut meshes, *min, *max);
    //         }
    //         EditorObject::Csg(csg) => {
    //             let material = materials.add(StandardMaterial {
    //                 base_color: Color::BLUE,
    //                 metallic: 0.9,
    //                 perceptual_roughness: 0.1,
    //                 ..Default::default()
    //             });

    //             add_csg(&mut commands, entity, material, &mut meshes, csg);
    //         }

    //     }
    //     info!("update brush mesh");
    // }
}

#[allow(clippy::type_complexity)]
pub fn update_brush_csg_system(
    mut commands: Commands,

    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,

    query: Query<&EditorObject>,
    query_changed: Query<Entity, Changed<EditorObject>>,
    query_cleanup: Query<(Entity, &Handle<Mesh>, &Handle<StandardMaterial>), With<CsgOutput>>,
) {
    if query_changed.is_empty() {
        return;
    }

    let start = Instant::now();
    // if any Brush has changed, first delete all existing CsgOutput entities including mesh and material resources
    for (entity, mesh, material) in &query_cleanup {
        info!("cleanup {:?} {:?}", mesh, material);
        meshes.remove(mesh);
        if let Some(material) = materials.remove(material) {
            if let Some(image) = material.base_color_texture {
                info!("cleanup {:?}", image);
                images.remove(image);
            }
        }

        commands.entity(entity).despawn();
    }

    let mut csgs = query
        .iter()
        .filter_map(|brush| match brush {
            EditorObject::Csg(csg) => Some(csg.clone()),
            EditorObject::Brush(brush) => brush.clone().try_into().ok(),
            _ => None,
        })
        .collect::<Vec<_>>();

    let Some(mut u) = csgs.pop() else {
        info!( "no Csg brushes");
        return;
    };

    for csg in csgs {
        u = csg::union(&u, &csg).unwrap();
    }

    u.invert();

    let material = materials.add(StandardMaterial {
        base_color: Color::BLUE,
        metallic: 0.9,
        perceptual_roughness: 0.1,
        ..Default::default()
    });

    let entity = commands
        .spawn()
        .insert(CsgOutput)
        .insert(Name::new("csg_output"))
        .insert(Wireframe)
        .id();
    add_csg(&mut commands, entity, material, &mut meshes, &u);

    info!("csg update: {:?}", start.elapsed());
}

pub fn track_primary_selection(
    selection: Res<Selection>,
    mut meshes: ResMut<Assets<Mesh>>,
    brush_query: Query<&EditorObject, Changed<EditorObject>>,
    mut query: Query<(&Handle<Mesh>, &mut Aabb), With<SelectionVis>>,
) {
    let Some(ref primary) = selection.primary else { return };
    let Ok(EditorObject::Brush(brush)) = brush_query.get(*primary) else { return };
    let Ok((vis,mut aabb)) = query.get_single_mut() else { return };
    let Some(mesh) = meshes.get_mut(vis) else { return };
    let Ok(csg): Result<csg::Csg, _> = brush.clone().try_into() else {return};
    *aabb = csg.get_aabb();
    *mesh = (&csg).into();
}

pub fn setup_selection_vis_system(
    mut command: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    command
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(Cube::default().into()),
            material: materials.add(Color::rgba(0.5, 0.5, 1.0, 0.2).into()),
            ..default()
        })
        .insert(SelectionVis)
        .insert(Name::new("selection"));
}

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
            Transform::from_xyz(0.0, 6.0, 0.0).looking_at(Vec3::ZERO, Vec3::X),
        ),
        (
            LOWER_WINDOW,
            None,
            Transform::from_xyz(-6.0, 0.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
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
                transform: *t,
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
    mut last_written_settings: Local<BTreeMap<String, EditorWindowSettings>>,
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
