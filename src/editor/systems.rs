use bevy::{
    input::mouse::MouseWheel,
    pbr::wireframe::Wireframe,
    prelude::{shape::Cube, *},
    render::primitives::Aabb,
    utils::Instant,
};

use super::{
    components::{CsgOutput, EditorObject, SelectionVis},
    resources::{self, Selection},
};
use crate::{
    csg::{self},
    editor::util::add_csg,
};

#[allow(clippy::too_many_arguments)]
pub fn editor_input_system(
    mut commands: Commands,

    mut offset: Local<Option<Vec3>>,

    editor_windows_2d: ResMut<resources::EditorWindows2d>,

    keycodes: Res<Input<KeyCode>>,
    mut mouse_wheel: EventReader<MouseWheel>,

    mut selection: ResMut<Selection>,

    mut query: Query<&mut EditorObject>,
) {
    if keycodes.just_pressed(KeyCode::K) {
        let entity = commands
            .spawn(EditorObject::MinMax(Vec3::splat(-1.0), Vec3::splat(1.0)))
            .id();

        selection.primary = Some(entity);
    }

    if keycodes.just_pressed(KeyCode::B) {
        let entity = commands
            .spawn(EditorObject::Brush(csg::Brush::default()))
            .id();

        selection.primary = Some(entity);
    }

    if keycodes.just_pressed(KeyCode::D) {
        if let Some(primary) = selection.primary {
            if let Ok(obj) = query.get(primary) {
                let entity = commands.spawn(obj.clone()).id();

                selection.primary = Some(entity);
            }
        }
    }

    if keycodes.just_pressed(KeyCode::L) {
        let entity = commands
            .spawn(EditorObject::Csg(
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
                    .spawn(EditorObject::Csg(
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

    // the remaining stuff only works in the 3d window
    if editor_windows_2d.focused.is_some() {
        return;
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
        .spawn(CsgOutput)
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
    let mut material: StandardMaterial = Color::rgba(0.5, 0.5, 1.0, 0.4).into();
    material.unlit = true;

    command
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(Cube::default().into()),
            material: materials.add(material),
            ..default()
        })
        .insert(SelectionVis)
        // .insert(Wireframe)
        .insert(Name::new("selection"));
}

pub fn load_save_editor_objects(
    mut commands: Commands,
    keycodes: Res<Input<KeyCode>>,
    existing_objects: Query<(Entity, &EditorObject), With<EditorObject>>,
) {
    if keycodes.just_pressed(KeyCode::F5) {
        let objects = existing_objects
            .iter()
            .map(|(_, obj)| obj)
            .collect::<Vec<_>>();
        if let Ok(file) = std::fs::File::create("scene.ron") {
            let _ = ron::ser::to_writer_pretty(file, &objects, default());
        }
    }

    if keycodes.just_pressed(KeyCode::F6) {
        // let objects = existing_objects.iter().map(|(_,obj)| obj).collect::<Vec<_>>();
        if let Ok(file) = std::fs::File::open("scene.ron") {
            let objects: Vec<EditorObject> = ron::de::from_reader(file).unwrap_or_default();

            for (entity, _) in existing_objects.iter() {
                commands.entity(entity).despawn();
            }
            for obj in objects {
                commands.spawn(obj);
            }
        }
    }
}
