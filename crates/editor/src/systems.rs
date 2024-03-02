use super::{
    components::{self, CsgOutput, CsgRepresentation},
    edit_commands::{add_brush, add_pointlight, duplicate_brush, remove_entity, EditCommands},
    resources,
};

use crate::{
    components::{BrushMaterialProperties, EditorObjectBrushBundle},
    util::spawn_csg_split,
    wsx,
};
use bevy::{
    pbr::wireframe::Wireframe,
    prelude::*,
    render::{mesh, view::RenderLayers},
    utils::{HashSet, Instant},
    window::PrimaryWindow,
};
use bevy_egui::EguiContexts;

#[cfg(feature = "external_deps")]
use bevy_mod_outline::OutlineMeshExt;
use serde::{Deserialize, Serialize};
use shared::render_layers;
use sstree::{SpatialBounds, SpatialIndex};
use std::{collections::BTreeSet, path::PathBuf};

pub fn setup(
    mut materials_res: ResMut<resources::Materials>,
    mut material_browser: ResMut<resources::MaterialBrowser>,
    mut asset_server: ResMut<AssetServer>,
    mut egui_context: EguiContexts,
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

    let mut material: StandardMaterial = Color::rgba(0.2, 1.0, 0.2, 0.6).into();
    material.unlit = true;
    material.cull_mode = None;
    materials_res.brush_clip_green = material_assets.add(material);

    let mut material: StandardMaterial = Color::rgba(1.0, 0.2, 0.2, 0.6).into();
    material.unlit = true;
    material.cull_mode = None;
    materials_res.brush_clip_red = material_assets.add(material);

    info!("loaded {} material defs", materials_res.material_defs.len());
}

#[allow(clippy::too_many_arguments)]
pub fn editor_input_system(
    mut commands: Commands,
    mut edit_commands: EditCommands,
    // mut windows: ResMut<Windows>,
    mut primary_query: Query<&mut Window, With<PrimaryWindow>>,
    keycodes: Res<ButtonInput<KeyCode>>,
    selection_query: Query<Entity, With<components::Selected>>,
    mut clip_state: ResMut<resources::ClipState>,
) {
    {
        let Ok(mut window) = primary_query.get_single_mut() else {
            return;
        };
        if keycodes.just_pressed(KeyCode::ShiftLeft) {
            window.cursor.grab_mode = bevy::window::CursorGrabMode::Confined;
        }
        if keycodes.just_released(KeyCode::ShiftLeft) {
            window.cursor.grab_mode = bevy::window::CursorGrabMode::None;
        }
    }
    if keycodes.pressed(KeyCode::ShiftLeft) {
        return;
    }

    let mut clear_selection = false;
    if keycodes.just_pressed(KeyCode::KeyB) {
        let res = edit_commands.apply(add_brush::Command { brush: default() });
        if let Err(err) = res {
            warn!("failed to add brush: {:?}", err);
        }

        clear_selection = true;

        info!("add brush");
    }

    if keycodes.just_pressed(KeyCode::KeyD) {
        if let Ok(primary) = selection_query.get_single() {
            let res = edit_commands.apply(duplicate_brush::Command {
                template_entity: primary,
            });
            if let Err(err) = res {
                warn!("failed to duplicate brush: {:?}", err);
            }
            clear_selection = true;
        }
    }

    if keycodes.just_pressed(KeyCode::KeyL) {
        let res = edit_commands.apply(add_pointlight::Command);
        if let Err(err) = res {
            warn!("failed to add point light: {:?}", err);
        }
        clear_selection = true;
    }

    if keycodes.just_pressed(KeyCode::KeyK) {
        commands
            .spawn((SpatialBundle::default(), components::EditablePoint))
            .with_children(|commands| {
                let mut offset = Vec3::ZERO;
                for _ in 0..20 {
                    commands.spawn((
                        PointLightBundle {
                            transform: Transform::from_translation(offset),
                            point_light: PointLight {
                                range: 2.0,
                                shadows_enabled: false,
                                ..default()
                            },
                            ..default()
                        },
                        RenderLayers::layer(render_layers::MAIN_3D),
                    ));
                    offset.x += 0.4;
                }
            });
    }

    // if keycodes.just_pressed(KeyCode::K) {
    //     let entity = commands
    //         .spawn(components::EditorObjectDirectionalLightBundle::default())
    //         .id();
    //     clear_selection = true;
    // }

    if keycodes.just_pressed(KeyCode::KeyX) {
        if let Ok(primary) = selection_query.get_single() {
            let res = edit_commands.apply(remove_entity::Command { entity: primary });
            if let Err(err) = res {
                warn!("failed to remove entity: {:?}", err);
            }
        }
    }

    if keycodes.just_pressed(KeyCode::KeyC) {
        clip_state.clip_mode = !clip_state.clip_mode;
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
            &mut Visibility,
        ),
        Changed<components::MaterialRef>,
    >,
) {
    if query_changed.is_empty() && materials_res.dirty_symlinks.is_empty() {
        return;
    }
    // asset_server.mark_unused_assets()
    for (entity, material_ref, mut material, mut visibility) in &mut query_changed {
        let Some(new_material) = materials_res.get(
            &material_ref.material_name,
            &mut materials,
            &mut asset_server,
        ) else {
            warn!(
                "material resource not found for {}",
                material_ref.material_name
            );
            continue;
        };
        // commands.entity(entity).insert(material);
        *material = new_material;
        // new brushes (with pink default material) start hidden to prevent flickering.
        *visibility = Visibility::Inherited;
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
        let Some(new_material) = materials_res.get(
            &material_ref.material_name,
            &mut materials,
            &mut asset_server,
        ) else {
            warn!(
                "material resource not found for {}",
                material_ref.material_name
            );
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

    mut query_changed: Query<(Entity, &CsgRepresentation, &Transform), With<components::CsgDirty>>,
    query_csg: Query<(
        &CsgRepresentation,
        &Transform,
        &components::BrushMaterialProperties,
    )>,
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
        affected.extend(spatial_index.query(csg_repr.bounds));
    }

    // re-create meshes for all affected brushes:
    // clip them against (potentially) overlapping brushes (according to spatial index)
    let num_affected = affected.len();
    for entity in affected {
        let Ok((csg_repr, transform, material_properties)) = query_csg.get(entity) else {
            error!("affected csg not found for {:?}", entity);
            continue;
        };

        debug!("csg changed: {:?}", entity);

        let others = spatial_index
            .query(csg_repr.bounds)
            .filter_map(|entry| {
                if entry == entity {
                    return None;
                }
                let (other_csg, _, _) = query_csg.get(entry).ok()?;
                let other_bsp = csg::Node::from_polygons(&other_csg.csg.polygons)?;
                if !csg_repr.csg.intersects_or_touches(&other_csg.csg) {
                    return None;
                }

                Some((other_bsp, entity < entry))
            })
            .collect::<Vec<_>>();

        // TODO: check if we should store bsp trees rather than Csg objects.in CsgRepresentation
        let mut bsp =
            csg::Node::from_polygons(&csg_repr.csg.polygons).expect("Node::from_polygons failed");

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
            let remove_children = children
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
        let mut new_children = spawn_csg_split(
            &mut commands,
            &mut meshes,
            &output_shape,
            transform.translation,
            &material_properties.materials[..],
        );

        if let Ok(mut processed) = processed_csg_query.get_mut(entity) {
            processed.bsp = bsp;
        } else {
            commands
                .entity(entity)
                .insert(components::ProcessedCsg { bsp });
        }

        const GENERATE_COLLISION_GEOMETRY: bool = true;
        if GENERATE_COLLISION_GEOMETRY {
            for (collider, origin) in output_shape.get_colliders() {
                // println!("collider: {:?}", collider);
                let entity = commands
                    .spawn(collider)
                    .insert(SpatialBundle::from_transform(Transform::from_translation(
                        origin - transform.translation,
                    )))
                    .insert(components::CsgOutput)
                    .id();
                new_children.push(entity);
            }
        }
        commands.entity(entity).push_children(&new_children);
    }

    for (entity, _, _) in &query_changed {
        commands.entity(entity).remove::<components::CsgDirty>();
    }

    if num_affected > 0 {
        info!("csg update: {} in {:?}", num_affected, start.elapsed());
    }
}

#[derive(Resource, Default)]
pub struct SelectionChangeTracking {
    selection: HashSet<Entity>,
}

// FIXME: make system independent from external dependency
#[cfg(not(feature = "external_deps"))]
pub fn track_primary_selection() {}

#[cfg(feature = "external_deps")]
pub fn track_primary_selection(
    // selection: Res<Selection>,
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
                warn!("no children: {:?}", entity);
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
                warn!("no children: {:?}", entity);
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

    changed_query: Query<(Entity, &CsgRepresentation, &Transform), Changed<CsgRepresentation>>,
    children_query: Query<&Children>,
    mut mesh_query: Query<
        (&mut Handle<Mesh>, &mut Transform),
        (Without<CsgOutput>, Without<CsgRepresentation>),
    >,
) {
    // info!("track");
    for (entity, csg_rep, transform) in &changed_query {
        if let Ok(children) = children_query.get(entity) {
            // 2d vis mesh already exists. just update.
            // info!("brush update");

            for child in children {
                if let Ok((old_mesh, mut mesh_transform)) = mesh_query.get_mut(*child) {
                    // meshes.remove(old_mesh.clone());
                    let (mut mesh, origin) = (&csg_rep.csg).into();
                    if let Some(old_mesh) = meshes.get_mut(old_mesh.clone()) {
                        #[cfg(features = "external_deps")]
                        {
                            let res = OutlineMeshExt::generate_outline_normals(&mut mesh);
                            if let Err(err) = res {
                                warn!(
                                    "failed to generate outline normals for {:?}: {:?}",
                                    child, err
                                );
                            }
                        }
                        *old_mesh = mesh;
                    }

                    // transform.translation = origin;
                    mesh_transform.translation = origin - transform.translation;

                    // *old_mesh = meshes.add(mesh);
                }
            }
        } else {
            let (mut mesh, origin) = (&csg_rep.csg).into();
            // transform.translation = origin;
            // info!("brush new");
            #[cfg(features = "external_deps")]
            {
                let res = OutlineMeshExt::generate_outline_normals(&mut mesh);
                if let Err(err) = res {
                    warn!("failed to generate outline normals for: {:?}", err);
                }
            }
            let mesh_entity = commands
                .spawn((
                    PbrBundle {
                        mesh: meshes.add(mesh),
                        material: materials_res.get_brush_2d_material(),
                        transform: Transform::from_translation(origin - transform.translation),
                        ..default()
                    },
                    render_layers::ortho_views(),
                    components::SelectionHighlighByMaterial,
                    Name::new("orthomesh"),
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
        Added<components::PointLightProperties>,
    >,
    directial_query: Query<
        (Entity, &components::DirectionalLightProperties, &Transform),
        Added<components::DirectionalLightProperties>,
    >,
    vis2d_query: Query<Entity, Added<components::EditablePoint>>,
    despawn_query: Query<
        Entity,
        (
            Or<(
                With<components::PointLightProperties>,
                With<components::DirectionalLightProperties>,
            )>,
            With<components::Despawn>,
        ),
    >,
) {
    for entity in &vis2d_query {
        let mesh: Mesh = mesh::shape::Icosphere {
            radius: 0.1,
            subdivisions: 2,
        }
        .try_into()
        .expect("Icosphere to mesh failed"); // FIXME: handle?
        let vis2d_entity = commands
            .spawn((
                PbrBundle {
                    mesh: meshes.add(mesh),
                    material: materials_res.get_brush_2d_material(),

                    ..default() // RenderLayers::from_layers(&[render_layers::SIDE_2D, render_layers::TOP_2D]),
                },
                render_layers::ortho_views(),
                components::SelectionHighlighByOutline,
                #[cfg(feature = "external_deps")]
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

        commands.entity(entity).add_child(vis2d_entity);
    }
    for (entity, light_props, _transform) in &query {
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

        commands.entity(entity).add_child(light_entity);
    }

    for (entity, light_props, _transform) in &directial_query {
        // directional 'sun' light
        let _half_size = light_props.half_size;

        let light_entity = commands
            .spawn((
                DirectionalLightBundle {
                    directional_light: DirectionalLight {
                        // shadow_projection: OrthographicProjection {
                        //     left: -half_size,
                        //     right: half_size,
                        //     bottom: -half_size,
                        //     top: half_size,
                        //     near: -10.0 * half_size,
                        //     far: 10.0 * half_size,
                        //     ..default()
                        // },

                        // shadows_enabled: true,
                        shadows_enabled: false,
                        ..default()
                    },
                    ..default()
                },
                RenderLayers::layer(render_layers::MAIN_3D),
                Name::new("bevy 3d PointLight"),
            ))
            .id();

        commands.entity(entity).add_child(light_entity);
    }
    for entity in &despawn_query {
        commands.entity(entity).despawn_recursive();
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

#[allow(clippy::type_complexity)]
pub fn track_brush_updates(
    mut commands: Commands,
    mut spatial_index: ResMut<sstree::SpatialIndex>,
    query_added: Query<
        (Entity, &components::CsgRepresentation),
        (Added<CsgRepresentation>, Without<components::EditUpdate>),
    >,
    mut query_modified: Query<
        (
            Entity,
            &mut csg::Brush,
            &mut components::CsgRepresentation,
            &mut Transform,
            &components::EditUpdate,
        ),
        Without<components::Despawn>,
    >,
    brush_despawn: Query<
        (Entity, &components::CsgRepresentation),
        (With<csg::Brush>, With<components::Despawn>),
    >,
) {
    let mut added_set = HashSet::new();
    for (entity, csg_repr) in &query_added {
        spatial_index.update(entity, None, csg_repr.bounds);
        added_set.insert(entity);
    }

    let mut spatial_dirty_set = HashSet::new();
    for (entity, mut old_brush, mut old_csg_repr, mut transform, edit_update) in &mut query_modified
    {
        if added_set.contains(&entity) {
            continue;
        }

        match edit_update {
            components::EditUpdate::BrushDrag { brush } => {
                let csg: Result<csg::Csg, _> = brush.clone().try_into();
                if let Ok(csg) = csg {
                    let (center, radius) = csg.bounding_sphere();
                    let bounds = SpatialBounds { center, radius };
                    // use center of spatial bounds as center for the whole entity.
                    // csg mesh and editor vis mesh need to be placed relative to it (ideally they use the same origin)
                    transform.translation = bounds.center;
                    spatial_index.update(entity, Some(old_csg_repr.bounds), bounds);
                    *old_brush = brush.clone();
                    *old_csg_repr = components::CsgRepresentation { csg, bounds };

                    spatial_dirty_set.extend(
                        spatial_index
                            .query(bounds)
                            .chain(spatial_index.query(old_csg_repr.bounds)),
                    );
                }
            }
        }
        commands.entity(entity).remove::<components::EditUpdate>();
    }

    for (entity, csg_repr) in &brush_despawn {
        commands.entity(entity).despawn_recursive();
        spatial_index.remove(entity, csg_repr.bounds);
        spatial_dirty_set.extend(spatial_index.query(csg_repr.bounds));
    }

    for dirty in spatial_dirty_set {
        commands.entity(dirty).insert(components::CsgDirty);
    }
}

#[derive(Serialize, Deserialize)]
enum ExternalEditorObject {
    Brush {
        brush: csg::Brush,
        material_properties: components::BrushMaterialProperties,
    },
    PointLight {
        translation: Vec3,
        light_properties: components::PointLightProperties,
    },
}

#[allow(clippy::too_many_arguments)]
pub fn load_save_editor_objects(
    mut commands: Commands,

    keycodes: Res<ButtonInput<KeyCode>>,
    brush_query: Query<(Entity, &csg::Brush, &components::BrushMaterialProperties)>,
    light_query: Query<(Entity, &components::PointLightProperties, &Transform)>,
    mut spatial_index: ResMut<SpatialIndex>,
    mut materials: ResMut<resources::Materials>,
) {
    if keycodes.just_pressed(KeyCode::F6) || keycodes.just_pressed(KeyCode::F7) {
        let despawn = brush_query
            .iter()
            .map(|(entity, _, _)| entity)
            .chain(light_query.iter().map(|(entity, _, _)| entity));

        for entity in despawn {
            commands.entity(entity).despawn_recursive();
        }
        // TODO: think again if this is smart
        spatial_index.clear();
    }

    if keycodes.just_pressed(KeyCode::F5) {
        let brushes =
            brush_query.iter().map(
                |(_, brush, material_properties)| ExternalEditorObject::Brush {
                    brush: brush.clone(),
                    material_properties: material_properties.clone(),
                },
            );

        let lights = light_query.iter().map(|(_, light_properties, transform)| {
            ExternalEditorObject::PointLight {
                translation: transform.translation,
                light_properties: light_properties.clone(),
            }
        });

        if let Ok(file) = std::fs::File::create("scene.ron") {
            let _ = ron::ser::to_writer_pretty(
                file,
                &brushes.chain(lights).collect::<Vec<_>>(),
                ron::ser::PrettyConfig::default(), // .indentor(" ".to_string())
                                                   // .compact_arrays(true),
            );
        }
        // if let Ok(mut file) = std::fs::File::create("scene.bin") {
        //     let v = flexbuffers::to_vec(&brushes.chain(lights).collect::<Vec<_>>()).unwrap();
        //     file.write_all(&v[..]).unwrap();
        // }
    }

    if keycodes.just_pressed(KeyCode::F6) {
        if let Ok(file) = std::fs::File::open("scene.ron") {
            let objects: Vec<ExternalEditorObject> = ron::de::from_reader(file).unwrap_or_default();

            for editor_object in objects {
                match editor_object {
                    ExternalEditorObject::Brush {
                        brush,
                        material_properties,
                    } => commands.spawn(
                        components::EditorObjectBrushBundle::from_brush(brush)
                            .with_material_properties(material_properties),
                    ),
                    ExternalEditorObject::PointLight {
                        translation,
                        light_properties,
                    } => commands.spawn(components::EditorObjectPointlightBundle {
                        spatial: SpatialBundle::from_transform(Transform::from_translation(
                            translation,
                        )),
                        light_properties,
                        ..default()
                    }),
                };
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
        info!("appearance map: {:?}", appearance_map);

        for mut brush in brushes {
            let materials = brush
                .appearances
                .iter()
                .map(|id| appearance_map.get(id).unwrap().clone())
                .collect();
            brush.appearances = (0..brush.planes.len() as i32).collect();

            commands.spawn(
                EditorObjectBrushBundle::from_brush(brush)
                    .with_material_properties(BrushMaterialProperties { materials }),
            );
        }
        materials.id_to_name_map = appearance_map;

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
            commands.spawn(components::EditorObjectPointlightBundle {
                spatial: SpatialBundle::from_transform(Transform::from_translation(pos)),
                light_properties: components::PointLightProperties {
                    shadows_enabled: false,
                    range: 5.0,
                },
                ..default()
            });
        }
    }
}

pub fn track_wireframe_system(
    mut commands: Commands,
    selected: Query<Entity, With<components::Selected>>,
    mut removed: RemovedComponents<components::Selected>,
    children: Query<&Children>,
    csg_with: Query<Entity, (With<components::CsgOutput>, With<Wireframe>)>,
    csg_without: Query<Entity, (With<components::CsgOutput>, Without<Wireframe>)>,
) {
    for e in &selected {
        let Ok(cs) = children.get(e) else { continue };
        for c in cs {
            let Ok(csg_ent) = csg_without.get(*c) else {
                continue;
            };
            commands.entity(csg_ent).insert(Wireframe);
        }
    }
    for e in removed.read() {
        let Ok(cs) = children.get(e) else { continue };
        for c in cs {
            let Ok(csg_ent) = csg_with.get(*c) else {
                continue;
            };
            commands.entity(csg_ent).remove::<Wireframe>();
        }
    }
}
