use super::{
    components::{self, CsgOutput, EditorObject, SelectionVis},
    resources::{self, Selection},
    CleanupCsgOutputEvent,
};
use crate::{
    csg,
    editor::{components::CsgCollisionOutput, util::spawn_csg_split},
    material, render_layers, sstree, wsx,
};
use bevy::{
    input::mouse::MouseWheel,
    prelude::{shape::Cube, *},
    render::{primitives::Aabb, view::RenderLayers},
    utils::Instant,
};
use std::path::PathBuf;

pub fn setup(
    mut materials_res: ResMut<resources::Materials>,
    mut material_browser: ResMut<resources::MaterialBrowser>,
    mut asset_server: ResMut<AssetServer>,
    mut egui_context: ResMut<bevy_egui::EguiContext>,
) {
    materials_res.material_defs =
        material::load_all_material_files(PathBuf::from("assets").join("materials"))
            .drain()
            .collect();
    materials_res.symlinks.insert(
        "appearance/test/con52_1".into(),
        "material/ground/bog".into(),
    );
    materials_res.symlinks.insert(
        "appearance/test/whiteconcret3".into(),
        "material/architecture/woodframe1".into(),
    );
    materials_res
        .id_to_name_map
        .insert(0, "appearance/test/whiteconcret3".into());
    material_browser.init_previews(
        materials_res.material_defs.values(),
        &mut asset_server,
        &mut egui_context,
    );
    info!("loaded {} material defs", materials_res.material_defs.len());
}

#[allow(clippy::too_many_arguments)]
pub fn editor_input_system(
    mut commands: Commands,

    mut offset: Local<Option<Vec3>>,

    editor_windows_2d: ResMut<resources::EditorWindows2d>,
    mut spatial_index: ResMut<resources::SpatialIndex>,

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
        let brush = csg::Brush::default();
        let csg: csg::Csg = brush.clone().try_into().unwrap();
        let (center, radius) = csg.bounding_sphere();

        let entity = commands
            .spawn((
                EditorObject::Brush(brush),
                components::BoundingSphere { center, radius },
            ))
            .id();

        info!("new brush: {:?}", entity);
        selection.primary = Some(entity);
        spatial_index.sstree.insert(entity, center, radius);
    }

    if keycodes.just_pressed(KeyCode::D) {
        if let Some(primary) = selection.primary {
            if let Ok(obj) = query.get(primary) {
                let entity = commands.spawn(obj.clone()).id();

                info!("duplicate brush: {:?} -> {:?}", primary, entity);
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
                    EditorObject::PointLight => (),
                }
            }
        }
    }

    // if mouse.any_pressed(MouseButton::Other(()))

    // if keycodes.just_pr
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn update_material_refs(
    mut materials_res: ResMut<resources::Materials>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut asset_server: ResMut<AssetServer>,

    mut query_changed: Query<
        (
            Entity,
            &components::MaterialRef,
            &mut Handle<StandardMaterial>,
        ),
        Changed<components::MaterialRef>,
    >,
) {
    if query_changed.is_empty() && materials_res.dirty_symlinks.is_empty() {
        return;
    }
    // asset_server.mark_unused_assets()
    for (entity, material_ref, mut material) in &mut query_changed {
        let Some(new_material) = materials_res.get(&material_ref.material_name,&mut materials, &mut asset_server) else {
            warn!( "material resource not found for {}", material_ref.material_name);
            continue;
        };
        // commands.entity(entity).insert(material);
        *material = new_material;
        info!("material ref changed {:?}", entity);
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn update_symlinked_materials(
    mut materials_res: ResMut<resources::Materials>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut asset_server: ResMut<AssetServer>,

    mut query_cleanup: Query<(
        Entity,
        &components::MaterialRef,
        &mut Handle<StandardMaterial>,
    )>,
) {
    if materials_res.dirty_symlinks.is_empty() {
        return;
    }
    debug!("dirty symlink: {:?}", materials_res.dirty_symlinks);

    for (entity, material_ref, mut material) in &mut query_cleanup {
        if !materials_res
            .dirty_symlinks
            .contains(&material_ref.material_name)
        {
            continue;
        }
        let Some(new_material) = materials_res.get(&material_ref.material_name,&mut materials, &mut asset_server) else {
            warn!( "material resource not found for {}", material_ref.material_name);
            continue;
        };
        // commands.entity(entity).insert(material);
        *material = new_material;
        info!("chnaged due to symlink {:?}", entity);
    }

    materials_res.dirty_symlinks.clear();
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn cleanup_brush_csg_system(
    mut commands: Commands,
    mut event_reader: EventReader<CleanupCsgOutputEvent>,
    mut meshes: ResMut<Assets<Mesh>>,
    query_changed: Query<(Entity, &EditorObject), Changed<EditorObject>>,
    query_cleanup: Query<(Entity, &Handle<Mesh>, &Handle<StandardMaterial>), With<CsgOutput>>,
    query_collision_cleanup: Query<Entity, With<components::CsgCollisionOutput>>,
) {
    if query_changed.is_empty() && event_reader.is_empty() {
        return;
    }

    for _ in event_reader.iter() {} // TODO: is this necessary?
                                    // if any Brush has changed, first delete all existing CsgOutput entities including mesh and material resources
    for (entity, mesh, material) in &query_cleanup {
        info!("cleanup {:?} {:?} {:?}", entity, mesh, material);
        meshes.remove(mesh);
        commands.entity(entity).despawn();
    }

    for entity in &query_collision_cleanup {
        commands.entity(entity).despawn();
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn create_brush_csg_system(
    mut commands: Commands,

    mut meshes: ResMut<Assets<Mesh>>,
    materials_res: ResMut<resources::Materials>,

    query: Query<&EditorObject>,
    query_changed: Query<(Entity, &EditorObject), Changed<EditorObject>>,
) {
    if query_changed.is_empty() {
        return;
    }

    for (entity, _) in query_changed.iter() {
        debug!("changed: {:?}", entity);
    }

    let start = Instant::now();

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

    spawn_csg_split(&mut commands, &materials_res, &mut meshes, &u);

    if false {
        for (collider, origin) in u.get_collision_polygons() {
            println!("collider: {:?}", collider);
            commands
                .spawn(collider)
                .insert(SpatialBundle::from_transform(Transform::from_translation(
                    origin,
                )))
                .insert(CsgCollisionOutput);
        }
    }
    debug!("csg update: {:?}", start.elapsed());
    // asset_server.free_unused_assets();
}

#[derive(Resource, Default)]
pub struct SelectionChangeTracking {
    primary: Option<Entity>,
}

pub fn track_primary_selection(
    selection: Res<Selection>,
    mut tracking: Local<SelectionChangeTracking>,
    mut meshes: ResMut<Assets<Mesh>>,
    brush_query: Query<(Entity, &EditorObject)>,
    brush_changed: Query<(), Changed<EditorObject>>,
    mut query: Query<(&Handle<Mesh>, &mut Aabb), With<SelectionVis>>,
) {
    if brush_changed.is_empty() && selection.primary == tracking.primary {
        return;
    }

    let Some(ref primary) = selection.primary else { return };
    let Ok((entity, EditorObject::Brush(brush))) = brush_query.get(*primary) else { return };
    let Ok((vis,mut aabb)) = query.get_single_mut() else { return };
    let Some(mesh) = meshes.get_mut(vis) else { return };
    let Ok(csg): Result<csg::Csg, _> = brush.clone().try_into() else {return};

    info!("selection vis changed: {:?}", entity);
    tracking.primary = selection.primary;
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
        .spawn(PbrBundle {
            mesh: meshes.add(Cube::default().into()),
            material: materials.add(material),
            ..default()
        })
        .insert(SelectionVis)
        // .insert(Wireframe)
        .insert(Name::new("selection"))
        .insert(RenderLayers::from_layers(&[
            render_layers::TOP_2D,
            render_layers::SIDE_2D,
        ]));
}

pub fn load_save_editor_objects(
    mut commands: Commands,
    mut event_writer: EventWriter<CleanupCsgOutputEvent>,

    keycodes: Res<Input<KeyCode>>,
    existing_objects: Query<(Entity, &EditorObject), With<EditorObject>>,
    mut materials: ResMut<resources::Materials>,
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

    if keycodes.just_pressed(KeyCode::F7) {
        // let objects = existing_objects.iter().map(|(_,obj)| obj).collect::<Vec<_>>();

        // let filename = &"t4.wsx";
        let filename = &"nav3.wsx";
        let (brushes, appearance_map) = wsx::load_brushes(filename);
        materials.id_to_name_map = appearance_map;
        for (entity, _) in existing_objects.iter() {
            commands.entity(entity).despawn();
        }
        for brush in &brushes[..] {
            commands.spawn(EditorObject::Brush(brush.clone()));
        }

        // TODO: do not load twice. Probably makes no difference, but I still hate it...
        let pointlights = wsx::load_pointlights(filename);
        for (pos, _range) in pointlights {
            commands
                .spawn(PointLightBundle {
                    point_light: PointLight {
                        range: 5.0, //range * 0.5,
                        shadows_enabled: false,
                        ..default()
                    },
                    transform: Transform::from_translation(pos),
                    ..default()
                })
                .insert(EditorObject::PointLight);
        }
    }

    if keycodes.just_pressed(KeyCode::F8) {
        for (entity, _) in existing_objects.iter() {
            commands.entity(entity).despawn();
        }
        event_writer.send(CleanupCsgOutputEvent);
    }
}
