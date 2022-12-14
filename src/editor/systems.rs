use super::{
    components::{
        self, CsgOutput, CsgRepresentation, EditorObject, EditorObjectBundle,
        EditorObjectOutputLink, PointLightProperties, SelectionVis,
    },
    resources::{self, Selection, SpatialIndex},
    CleanupCsgOutputEvent,
};
use crate::{
    csg,
    editor::{
        components::{CsgCollisionOutput, EditorObjectBrushBundle},
        util::spawn_csg_split,
    },
    material, render_layers, sstree, wsx,
};
use bevy::{
    input::mouse::MouseWheel,
    pbr::{wireframe::Wireframe, NotShadowCaster, NotShadowReceiver},
    prelude::{shape::Cube, *},
    render::{
        mesh,
        primitives::{Aabb, Sphere},
        view::RenderLayers,
    },
    utils::{HashSet, Instant},
};
use std::{collections::BTreeSet, path::PathBuf};

pub fn setup(
    mut materials_res: ResMut<resources::Materials>,
    mut material_browser: ResMut<resources::MaterialBrowser>,
    mut asset_server: ResMut<AssetServer>,
    mut egui_context: ResMut<bevy_egui::EguiContext>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
) {
    materials_res.material_defs =
        material::load_all_material_files(PathBuf::from("assets").join("materials"))
            .drain()
            .collect();
    materials_res.symlinks.insert(
        "appearance/test/con52_1".into(),
        "material/floors/bathroomtile1".into(),
    );

    materials_res.symlinks.insert(
        "appearance/test/whiteconcret3".into(),
        "material/floors/green-shower-tile1".into(),
    );
    materials_res
        .id_to_name_map
        .insert(0, "appearance/test/whiteconcret3".into());
    material_browser.init_previews(
        materials_res.material_defs.values(),
        &mut asset_server,
        &mut egui_context,
    );
    let mut material: StandardMaterial = Color::rgba(0.5, 0.5, 1.0, 0.2).into();
    material.unlit = true;
    material.cull_mode = None;
    materials_res.brush_2d = material_assets.add(material);

    let mut material: StandardMaterial = Color::rgba(1.0, 0.5, 0.5, 0.4).into();
    material.unlit = true;
    material.cull_mode = None;
    materials_res.brush_2d_selected = material_assets.add(material);
    info!("loaded {} material defs", materials_res.material_defs.len());
}

#[allow(clippy::too_many_arguments)]
pub fn editor_input_system(
    mut commands: Commands,

    editor_windows_2d: ResMut<resources::EditorWindows2d>,
    keycodes: Res<Input<KeyCode>>,
    mut selection: ResMut<Selection>,
    query: Query<&EditorObject>,
) {
    if keycodes.just_pressed(KeyCode::B) {
        let entity = commands
            .spawn(EditorObjectBrushBundle::from_brush(default()))
            .id();

        info!("new brush: {:?}", entity);
        selection.primary = Some(entity);
        // spatial_index.sstree.insert(entity, center, radius);
    }

    if keycodes.just_pressed(KeyCode::D) {
        if let Some(primary) = selection.primary {
            if let Ok(EditorObject::Brush(brush)) = query.get(primary) {
                let entity = commands
                    .spawn(EditorObjectBrushBundle::from_brush(brush.clone()))
                    .id();
                info!("duplicate brush: {:?} -> {:?}", primary, entity);
                selection.primary = Some(entity);
            }
        }
    }

    if keycodes.just_pressed(KeyCode::L) {
        let entity = commands
            .spawn((
                SpatialBundle::default(),
                EditorObjectBundle {
                    editor_object: EditorObject::PointLight(components::PointLightProperties {
                        shadows_enabled: true,
                        ..default()
                    }),
                    ..default()
                },
            ))
            .id();

        selection.primary = Some(entity);
    }
    // the remaining stuff only works in the 3d window
    if editor_windows_2d.focused.is_some() {
        return;
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn update_material_refs_system(
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
        debug!("material ref changed {:?}", entity);
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn update_symlinked_materials_system(
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

    query: Query<&components::CsgRepresentation>,
    query_changed: Query<Entity, Changed<components::CsgRepresentation>>,
) {
    if query_changed.is_empty() {
        return;
    }

    for entity in query_changed.iter() {
        debug!("changed: {:?}", entity);
    }

    let start = Instant::now();

    // let mut csgs = query
    //     .iter()
    //     .filter_map(|brush| match brush {
    //         EditorObject::Csg(csg) => Some(csg.clone()),
    //         EditorObject::Brush(brush) => brush.clone().try_into().ok(),
    //         _ => None,
    //     })
    //     .collect::<Vec<_>>();

    let mut csgs = query.iter().map(|x| &x.csg).collect::<Vec<_>>();

    let Some(mut u) = csgs.pop().cloned() else {
        info!( "no Csg brushes");
        return;
    };

    for csg in csgs {
        u = csg::union(&u, csg).unwrap();
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

pub fn create_brush_csg_system_inc(
    mut commands: Commands,
    spatial_index: Res<SpatialIndex>,

    mut meshes: ResMut<Assets<Mesh>>,
    materials_res: ResMut<resources::Materials>,

    mut query_changed: Query<(Entity, &CsgRepresentation), Changed<components::CsgRepresentation>>,
    query_csg: Query<&CsgRepresentation>,
    mut query_csg_out: Query<&mut EditorObjectOutputLink>,
) {
    let start = Instant::now();

    // Incrementally update csg. It may help to think about the brushes forming a graph, where
    // brushes are connected if their bounds overlap (according to the spatial index).
    //
    // Basically there are three sets of brushes to consider for the update:
    // 1. Changed by user interaction
    // 2. Overlapping with brushes in set 1.
    // 3. Overlapping with brushes in set 2 (but not set 1)
    // (4. all the rest, we don't care about them)
    //
    // What we need to do with them:
    // Set 1:
    //  - Obviously we need to re-create their meshes, as they were changed
    //  - need to be clipped against brushes in set 2
    // Set 2:
    //  - meshes also need to be re-created (but potentially they are unchanged)
    //  - need to be clipped against brushes in set 3 (as they can touch them), and set 1
    // Set 3:
    //  - We need to query for them to clip set 2 brushes
    //  - we must *not* re-create their meshes: although brushes in set 2 are re-clipped,
    //    their actual dimensions do not change and thus cannot affect the meshes of set 3 brushes.
    //    Formally they comprise the 'outer boundary' for traversing the 'brush overlap graph',
    //    which effectively makes the incremental update possible.
    //
    // NOTE: set 3 is only used for illustration here, it is never explicitly generated. Set 1&2 brushes
    //       always query their individual overalpping set.

    // find set of brushes (potentially) affected by recent changes:
    // 1. changed brushes
    // 2. brushes overlapping (approximately, according to spatial index) with changed brushes
    // TODO: brushes overlapping the old geometry of changed brushes, otherwise fast moving (teleporting) brushes
    // can leave holes etc.
    let mut affected = query_changed.iter().map(|(e, _)| e).collect::<HashSet<_>>();
    for (_entity, csg_repr) in &mut query_changed {
        let mut out = Vec::new();
        spatial_index.sstree.find_entries_within_radius(
            &csg_repr.center,
            csg_repr.radius,
            &mut out,
        );
        affected.extend(out.drain(..).map(|entry| entry.payload));
    }

    // re-create meshes for all affected brushes:
    // clip them against (potentially) overlapping brushes (according to spatial index)
    let num_affected = affected.len();
    for entity in affected {
        let Ok(csg_repr) = query_csg.get(entity) else {
            error!("affected csg not found for {:?}", entity);
            continue;
        };

        debug!("csg changed: {:?}", entity);
        let mut out = Vec::new();
        spatial_index.sstree.find_entries_within_radius(
            &csg_repr.center,
            csg_repr.radius,
            &mut out,
        );
        // TODO: check if we should store bsp trees rather than Csg objects.in CsgRepresentation
        let mut bsp =
            csg::Node::from_polygons(&csg_repr.csg.polygons).expect("Node::from_polygons failed");

        let others = out
            .iter()
            .filter_map(|entry| {
                if entry.payload == entity {
                    return None;
                }
                let other_csg = query_csg.get(entry.payload).ok()?;
                let other_bsp = csg::Node::from_polygons(&other_csg.csg.polygons)?;
                if !csg_repr.csg.intersects_or_touches(&other_csg.csg) {
                    return None;
                }

                Some((other_bsp, entity < entry.payload))
            })
            .collect::<Vec<_>>();
        // clip to overlapping brushes
        for (other_bsp, _) in &others {
            bsp.clip_to(other_bsp);
        }

        // invert (since we want them to be hollow)
        bsp.invert();
        // re-clip against overlapping brushes to remove overlapping coplanar faces (since the normal is now flipped)
        for (other_bsp, clip_inverse) in &others {
            // but only if their entity-id is higher than ours (this is an arbitary criterion to resolve ties,
            // i.e. one of the brushes must loose in this step.)
            if !*clip_inverse {
                continue;
            }
            bsp.clip_to(other_bsp);
        }
        // TODO: here it probably would help to check if the csg output actually changed before tearing down the meshes...
        let mut csg_output = query_csg_out.get_mut(entity).expect("missing csg_out"); // should be impossible if CsgOutputLink is always created in bundle with CsgRepresentation

        for entity in csg_output.entities.drain(..) {
            commands.entity(entity).despawn();
        }

        csg_output.entities = spawn_csg_split(
            &mut commands,
            &materials_res,
            &mut meshes,
            &csg::Csg::from_polygons(bsp.all_polygons()),
        );
    }

    if num_affected > 0 {
        info!("csg update: {} in {:?}", num_affected, start.elapsed());
    }
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

    debug!("selection vis changed: {:?}", entity);
    tracking.primary = selection.primary;
    *aabb = csg.get_aabb();
    *mesh = (&csg).into();
}

pub fn setup_selection_vis_system(
    mut command: Commands,
    materials_res: Res<resources::Materials>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    command
        .spawn(PbrBundle {
            mesh: meshes.add(Cube::default().into()),
            material: materials_res.get_brush_2d_selected_material(),
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

#[allow(clippy::type_complexity)]
pub fn track_2d_vis_system(
    mut command: Commands,
    materials_res: Res<resources::Materials>,
    mut meshes: ResMut<Assets<Mesh>>,

    new_query: Query<
        (Entity, &CsgRepresentation),
        (Changed<CsgRepresentation>, Without<Handle<Mesh>>),
    >,
    changed_query: Query<(Entity, &CsgRepresentation, &Handle<Mesh>), Changed<CsgRepresentation>>,
) {
    for (entity, csg_rep) in &new_query {
        info!("new");

        let mesh: Mesh = (&csg_rep.csg).into();

        command
            .entity(entity)
            .insert(PbrBundle {
                mesh: meshes.add(mesh),
                material: materials_res.get_brush_2d_material(),
                ..default()
            })
            // .insert(Wireframe)
            // .insert(RenderLayers::from_layers(&[
            //     render_layers::TOP_2D,
            //     render_layers::SIDE_2D,
            // ]))
            .insert(NotShadowCaster)
            .insert(NotShadowReceiver);
    }

    for (_entity, csg_rep, mesh_handle) in &changed_query {
        let mesh: Mesh = (&csg_rep.csg).into();

        let Some(old_mesh) = meshes.get_mut(mesh_handle) else {
                    error!( "could not lookup existing mesh");
                    continue;
                };
        *old_mesh = mesh;
    }
}

pub fn track_lights_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    materials_res: Res<resources::Materials>,
    query: Query<(Entity, &EditorObject, &Transform), (With<EditorObject>, Without<Handle<Mesh>>)>,
    // query_changed: Query<(Entity, &EditorObject), Without<Handle<Mesh>>>,
) {
    for (entity, editor_object, transform) in &query {
        // if !matches!(editor_object, EditorObject::PointLight(_)) {
        //     continue;
        // }

        let components::EditorObject::PointLight(light_props) = editor_object else {
            continue;
        };

        commands
            .entity(entity)
            .insert((
                meshes.add(
                    mesh::shape::Icosphere {
                        radius: 0.1,
                        subdivisions: 2,
                    }
                    .into(),
                ),
                materials_res.get_brush_2d_material(),
                // RenderLayers::from_layers(&[render_layers::SIDE_2D, render_layers::TOP_2D]),
            ))
            .insert(NotShadowCaster)
            .insert(NotShadowReceiver);

        commands.spawn((
            PointLightBundle {
                transform: *transform,
                point_light: PointLight {
                    shadows_enabled: light_props.shadows_enabled,
                    range: light_props.range.unwrap_or_else(|| default()),
                    ..default()
                },
                ..default()
            },
            RenderLayers::layer(render_layers::MAIN_3D),
        ));
    }
}

pub fn track_spatial_index_system(
    mut spatial_index: ResMut<resources::SpatialIndex>,
    query_added: Query<(Entity, &CsgRepresentation), Added<CsgRepresentation>>,
    query_modified: Query<(Entity, &CsgRepresentation), Changed<CsgRepresentation>>,
) {
    let mut added_set = HashSet::new();
    for (entity, csg_repr) in &query_added {
        spatial_index
            .sstree
            .insert(entity, csg_repr.center, csg_repr.radius);
        added_set.insert(entity);
    }
    for (entity, csg_repr) in &query_modified {
        if added_set.contains(&entity) {
            continue;
        }

        let Some(_entry)  = spatial_index
            .sstree
            .remove_if(&csg_repr.center, csg_repr.radius, |e| *e == entity) else {
                error!( "failed to remove brush from spatial index for update");
                // info!( "{:?} {} {:?}", csg_repr.center, csg_repr.radius, entity);
                // info!( "{:?}", spatial_index.sstree);
                panic!( "aborting.");
                continue;
            };
        debug!(
            "update: {:?} {} -> {:?} {}",
            _entry.center, _entry.radius, csg_repr.center, csg_repr.radius
        );
        spatial_index
            .sstree
            .insert(entity, csg_repr.center, csg_repr.radius);
    }
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

        let materials = &mut *materials;
        // let filename = &"t4.wsx";
        // let filename = &"x8.wsx";
        let filename = &"nav3.wsx";
        let (brushes, appearance_map) = wsx::load_brushes(filename);
        materials.id_to_name_map = appearance_map;
        for (entity, _) in existing_objects.iter() {
            commands.entity(entity).despawn();
        }
        for brush in brushes {
            commands.spawn(EditorObjectBrushBundle::from_brush(brush));
        }

        let appearance_names = materials.id_to_name_map.values().collect::<BTreeSet<_>>();
        // let mut material_names = materials.material_defs.keys();
        let mut material_names = [
            "material/floors/bathroomtile2",
            "material/floors/bathroomtile1",
            "material/floors/rich-brown-tile",
            "material/floors/modern-tile1",
            "material/floors/green-shower-tile1",
            "material/floors/green-ceramic-tiles",
            "material/floors/industrial-tile1",
            "material/floors/diamond-inlay-tile",
            "material/floors/cheap-old-linoleum",
            "material/floors/gross-dirty-tiles",
            "material/floors/bathroomtile2",
            "material/floors/bathroomtile1",
            "material/floors/rich-brown-tile",
            "material/floors/modern-tile1",
            "material/floors/green-shower-tile1",
            "material/floors/green-ceramic-tiles",
            "material/floors/industrial-tile1",
            "material/floors/diamond-inlay-tile",
            "material/floors/cheap-old-linoleum",
            "material/floors/gross-dirty-tiles",
        ]
        .iter();
        for name in appearance_names {
            match materials.symlinks.entry(name.clone()) {
                bevy::utils::hashbrown::hash_map::Entry::Vacant(e) => {
                    e.insert(material_names.next().unwrap().to_string());
                }
                bevy::utils::hashbrown::hash_map::Entry::Occupied(_) => (),
            }
        }

        // TODO: do not load twice. Probably makes no difference, but I still hate it...
        let pointlights = wsx::load_pointlights(filename);
        for (pos, _range) in pointlights {
            commands.spawn((
                SpatialBundle::from_transform(Transform::from_translation(pos)),
                EditorObjectBundle {
                    editor_object: EditorObject::PointLight(PointLightProperties {
                        range: Some(10.0),
                        ..default()
                    }),
                    ..default()
                },
            ));
        }
    }

    if keycodes.just_pressed(KeyCode::F8) {
        for (entity, _) in existing_objects.iter() {
            commands.entity(entity).despawn();
        }
        event_writer.send(CleanupCsgOutputEvent);
    }
}
