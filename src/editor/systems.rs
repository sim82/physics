use super::{
    components::{self, CsgOutput, CsgRepresentation, EditorObjectBundle, PointLightProperties},
    resources::{self, Selection, SpatialIndex},
    CleanupCsgOutputEvent,
};
use crate::{
    csg,
    editor::{
        components::{CsgCollisionOutput, EditorObjectBrushBundle},
        util::spawn_csg_split,
    },
    material, render_layers, wsx,
};
use bevy::{
    pbr::{NotShadowCaster, NotShadowReceiver},
    prelude::{shape::Cube, *},
    render::{mesh, primitives::Aabb, view::RenderLayers},
    utils::{HashSet, Instant},
};
use bevy_mod_outline::OutlineMeshExt;
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

    keycodes: Res<Input<KeyCode>>,
    mut selection: ResMut<Selection>,
    query: Query<&csg::Brush>,
    selection_query: Query<Entity, With<components::Selected>>,
) {
    let mut clear_selection = false;
    if keycodes.just_pressed(KeyCode::B) {
        let entity = commands
            .spawn((
                EditorObjectBrushBundle::from_brush(default()),
                components::Selected,
            ))
            .id();
        clear_selection = true;
        info!("new brush: {:?}", entity);
        // selection.primary = Some(entity);
        // spatial_index.sstree.insert(entity, center, radius);
    }

    if keycodes.just_pressed(KeyCode::D) {
        if let Ok(primary) = selection_query.get_single() {
            if let Ok(brush) = query.get(primary) {
                let entity = commands
                    .spawn((
                        EditorObjectBrushBundle::from_brush(brush.clone()),
                        components::Selected,
                    ))
                    .id();
                info!("duplicate brush: {:?} -> {:?}", primary, entity);
                // selection.primary = Some(entity);
                clear_selection = true;
            }
        }
    }

    if keycodes.just_pressed(KeyCode::L) {
        let entity = commands
            .spawn((
                SpatialBundle::default(),
                EditorObjectBundle {
                    // editor_object: EditorObject::PointLight(components::PointLightProperties {
                    //     shadows_enabled: true,
                    //     ..default()
                    // }),
                    ..default()
                },
                components::PointLightProperties {
                    shadows_enabled: true,
                    ..default()
                },
                Name::new("PointLight"),
                components::Selected,
            ))
            .id();
        clear_selection = true;

        // selection.primary = Some(entity);
    }

    if clear_selection {
        for entity in &selection_query {
            commands.entity(entity).remove::<components::Selected>();
        }
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

#[allow(clippy::too_many_arguments)]
pub fn create_brush_csg_system_inc(
    mut commands: Commands,
    spatial_index: Res<SpatialIndex>,

    mut meshes: ResMut<Assets<Mesh>>,
    materials_res: ResMut<resources::Materials>,

    mut query_changed: Query<
        (Entity, &CsgRepresentation, &Transform),
        Changed<components::CsgRepresentation>,
    >,
    query_csg: Query<(&CsgRepresentation, &Transform)>,
    query_children: Query<&Children>,
    query_csg_output: Query<(), With<components::CsgOutput>>,
    mut processed_csg_query: Query<&mut components::ProcessedCsg>,
    // mut query_csg_out: Query<&mut EditorObjectOutputLink>,
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
    let mut affected = query_changed
        .iter()
        .map(|(e, _, _)| e)
        .collect::<HashSet<_>>();
    for (_entity, csg_repr, &_transform) in &mut query_changed {
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
        let Ok((csg_repr, transform)) = query_csg.get(entity) else {
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
                let (other_csg, _) = query_csg.get(entry.payload).ok()?;
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
        // let mut csg_output = query_csg_out.get_mut(entity).expect("missing csg_out"); // should be impossible if CsgOutputLink is always created in bundle with CsgRepresentation

        // for entity in csg_output.entities.drain(..) {
        //     commands.entity(entity).despawn();
        // }

        if let Ok(children) = query_children.get(entity) {
            let mut entity_commands = commands.entity(entity);
            let mut remove_children = children
                .iter()
                .cloned()
                .filter(|child| query_csg_output.contains(*child))
                .collect::<Vec<_>>();
            commands.entity(entity).remove_children(&remove_children);
            for child in remove_children {
                commands.entity(child).despawn();
            }
        }

        let output_shape = csg::Csg::from_polygons(bsp.all_polygons());
        let new_children = spawn_csg_split(
            &mut commands,
            &materials_res,
            &mut meshes,
            &output_shape,
            transform.translation,
        );
        commands.entity(entity).push_children(&new_children);

        if let Ok(mut processed) = processed_csg_query.get_mut(entity) {
            processed.bsp = bsp;
        } else {
            commands
                .entity(entity)
                .insert(components::ProcessedCsg { bsp });
        }

        // const GENERATE_COLLISION_GEOMETRY: bool = true;
        // if GENERATE_COLLISION_GEOMETRY {
        //     for (collider, origin) in output_shape.get_collision_polygons() {
        //         // println!("collider: {:?}", collider);
        //         let entity = commands
        //             .spawn(collider)
        //             .insert(SpatialBundle::from_transform(Transform::from_translation(
        //                 origin,
        //             )))
        //             .insert(CsgCollisionOutput)
        //             .id();
        //         csg_output.entities.push(entity);
        //     }
        // }
    }

    if num_affected > 0 {
        info!("csg update: {} in {:?}", num_affected, start.elapsed());
    }
}

#[derive(Resource, Default)]
pub struct SelectionChangeTracking {
    selection: HashSet<Entity>,
}

// pub fn track_primary_selection(
//     selection: Res<Selection>,
//     materials_res: Res<resources::Materials>,
//     mut tracking: Local<SelectionChangeTracking>,
//     mut material_query: Query<&mut Handle<StandardMaterial>>,
// ) {
//     if selection.primary == tracking.primary {
//         return;
//     }

//     // reset material for old selecton
//     if let Some(mut material) = tracking
//         .primary
//         .and_then(|old_selection| material_query.get_mut(old_selection).ok())
//     {
//         *material = materials_res.get_brush_2d_material();
//     }

//     if let Some(mut material) = selection
//         .primary
//         .and_then(|new_selection| material_query.get_mut(new_selection).ok())
//     {
//         *material = materials_res.get_brush_2d_selected_material();
//     }

//     tracking.primary = selection.primary;
// }

pub fn track_primary_selection(
    // selection: Res<Selection>,
    mut commands: Commands,
    materials_res: Res<resources::Materials>,
    mut tracking: Local<SelectionChangeTracking>,

    selection_query: Query<Entity, With<components::Selected>>,
    mut outline_query: Query<
        &mut bevy_mod_outline::OutlineVolume,
        With<components::SelectionHighlighByOutline>,
    >,
    mut material_query: Query<
        &mut Handle<StandardMaterial>,
        With<components::SelectionHighlighByMaterial>,
    >,
    children_query: Query<&Children>,
) {
    // TODO: this is a brute force PoC with some major inefficiencies.
    // use change detection on Selected components
    let new_selection = selection_query.iter().collect::<HashSet<_>>();
    if new_selection == tracking.selection {
        return;
    }
    info!("selection: {:?} {:?}", new_selection, tracking.selection);

    {
        let to_default_material = tracking
            .selection
            .difference(&new_selection)
            .collect::<HashSet<_>>();
        let to_selected_material = new_selection
            .difference(&tracking.selection)
            .collect::<HashSet<_>>();

        for entity in to_default_material {
            let Ok(children) = children_query.get(*entity) else {
                warn!( "no children: {:?}", entity);
                continue;
            };
            for child in children {
                if let Ok(mut material) = material_query.get_mut(*child) {
                    *material = materials_res.get_brush_2d_material();
                } else if let Ok(mut outline) = outline_query.get_mut(*child) {
                    outline.colour = Color::BLUE;
                    outline.width = 2.0;
                }
            }
        }
        for entity in to_selected_material {
            let Ok(children) = children_query.get(*entity) else {
                warn!( "no children: {:?}", entity);
                continue;
            };

            for child in children {
                if let Ok(mut material) = material_query.get_mut(*child) {
                    *material = materials_res.get_brush_2d_selected_material();
                } else if let Ok(mut outline) = outline_query.get_mut(*child) {
                    outline.colour = Color::RED;
                    outline.width = 4.0;
                }
            }
        }
    }
    tracking.selection = new_selection;
}

#[allow(clippy::type_complexity)]
pub fn track_2d_vis_system(
    mut commands: Commands,
    materials_res: Res<resources::Materials>,
    mut meshes: ResMut<Assets<Mesh>>,

    mut changed_query: Query<
        (Entity, &CsgRepresentation, &mut Transform),
        Changed<CsgRepresentation>,
    >,
    children_query: Query<&Children>,
    mut mesh_query: Query<&mut Handle<Mesh>, Without<CsgOutput>>,
) {
    // info!("track");
    for (entity, csg_rep, mut transform) in &mut changed_query {
        if let Ok(children) = children_query.get(entity) {
            // 2d vis mesh already exists. just update.
            info!("brush update");

            for child in children {
                if let Ok(mut old_mesh) = mesh_query.get_mut(*child) {
                    // meshes.remove(old_mesh.clone());
                    let (mut mesh, origin) = (&csg_rep.csg).into();
                    if let Some(old_mesh) = meshes.get_mut(&old_mesh) {
                        OutlineMeshExt::generate_outline_normals(&mut mesh);
                        *old_mesh = mesh;
                    }

                    transform.translation = origin;

                    // *old_mesh = meshes.add(mesh);
                }
            }
        } else {
            let (mut mesh, origin) = (&csg_rep.csg).into();
            transform.translation = origin;
            info!("brush new");
            OutlineMeshExt::generate_outline_normals(&mut mesh);
            let mesh_entity = commands
                .spawn((
                    PbrBundle {
                        mesh: meshes.add(mesh),
                        material: materials_res.get_brush_2d_material(),
                        ..default()
                    },
                    render_layers::ortho_views(),
                    components::SelectionHighlighByMaterial,
                ))
                .id();
            commands.entity(entity).add_child(mesh_entity);
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn track_lights_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    materials_res: Res<resources::Materials>,
    query: Query<
        (Entity, &components::PointLightProperties, &Transform),
        (With<components::PointLightProperties>, Without<Children>),
    >,
    // query_changed: Query<(Entity, &EditorObject), Without<Handle<Mesh>>>,
) {
    for (entity, light_props, transform) in &query {
        // if !matches!(editor_object, EditorObject::PointLight(_)) {
        //     continue;
        // }

        let light_entity = commands
            .spawn((
                PointLightBundle {
                    point_light: PointLight {
                        shadows_enabled: light_props.shadows_enabled,
                        range: light_props.range,
                        ..default()
                    },
                    ..default()
                },
                RenderLayers::layer(render_layers::MAIN_3D),
                Name::new("bevy 3d PointLight"),
            ))
            .id();

        let vis2d_entity = commands
            .spawn((
                PbrBundle {
                    mesh: meshes.add(
                        mesh::shape::Icosphere {
                            radius: 0.1,
                            subdivisions: 2,
                        }
                        .into(),
                    ),
                    material: materials_res.get_brush_2d_material(),

                    ..default() // RenderLayers::from_layers(&[render_layers::SIDE_2D, render_layers::TOP_2D]),
                },
                render_layers::ortho_views(),
                components::SelectionHighlighByOutline,
                bevy_mod_outline::OutlineBundle {
                    outline: bevy_mod_outline::OutlineVolume {
                        colour: Color::BLUE,
                        visible: true,
                        width: 2.0,
                    },
                    ..default()
                },
                Name::new("2dvis Mesh"),
            ))
            .id();

        commands
            .entity(entity)
            .add_child(light_entity)
            .add_child(vis2d_entity);
        // .insert(NotShadowCaster)
        // .insert(NotShadowReceiver)
        // .insert(EditorObjectLinkedBevyTransform(light_entity))
        // .add_child(light_entity);
    }
}

// pub fn track_linked_transforms_system(
//     query: Query<(&Transform, &EditorObjectLinkedBevyTransform)>,
//     mut transform_query: Query<&mut Transform, Without<EditorObjectLinkedBevyTransform>>,
// ) {
//     for (src_transform, linked) in &query {
//         if let Ok(mut dest_transform) = transform_query.get_mut(linked.0) {
//             *dest_transform = *src_transform;
//         }
//     }
// }

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
                // continue;
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
    // existing_objects: Query<(Entity, &csg::Brush), With<csg::Brush>>,
    delete_query: Query<Entity, Or<(With<csg::Brush>, With<components::PointLightProperties>)>>,
    mut materials: ResMut<resources::Materials>,
) {
    if keycodes.just_pressed(KeyCode::F5) {
        // let objects = existing_objects
        //     .iter()
        //     .map(|(_, obj)| obj)
        //     .collect::<Vec<_>>();
        // if let Ok(file) = std::fs::File::create("scene.ron") {
        //     let _ = ron::ser::to_writer_pretty(file, &objects, default());
        // }
        todo!()
    }

    if keycodes.just_pressed(KeyCode::F6) {
        // // let objects = existing_objects.iter().map(|(_,obj)| obj).collect::<Vec<_>>();
        // if let Ok(file) = std::fs::File::open("scene.ron") {
        //     let objects: Vec<EditorObject> = ron::de::from_reader(file).unwrap_or_default();

        //     for (entity, _) in existing_objects.iter() {
        //         commands.entity(entity).despawn();
        //     }
        //     for editor_object in objects {
        //         match editor_object {
        //             EditorObject::None => todo!(),
        //             EditorObject::Brush(brush) => {
        //                 commands.spawn(EditorObjectBrushBundle::from_brush(brush))
        //             }
        //             EditorObject::PointLight(_) => commands.spawn(EditorObjectBundle {
        //                 editor_object,
        //                 ..default()
        //             }),
        //         };
        //     }
        // }
        todo!();
    }

    if keycodes.just_pressed(KeyCode::F7) {
        // let objects = existing_objects.iter().map(|(_,obj)| obj).collect::<Vec<_>>();

        let materials = &mut *materials;
        // let filename = &"t4.wsx";
        let filename = &"x8.wsx";
        // let filename = &"nav3.wsx";
        let (brushes, appearance_map) = wsx::load_brushes(filename);
        materials.id_to_name_map = appearance_map;
        for entity in delete_query.iter() {
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
                EditorObjectBundle { ..default() },
                PointLightProperties { ..default() },
            ));
        }
    }

    if keycodes.just_pressed(KeyCode::F8) {
        for entity in delete_query.iter() {
            commands.entity(entity).despawn();
        }
        event_writer.send(CleanupCsgOutputEvent);
    }
}
