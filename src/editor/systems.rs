use bevy::{
    input::mouse::MouseWheel,
    pbr::wireframe::Wireframe,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    utils::Instant,
};

use super::{
    components::{Brush, CsgOutput},
    resources::Selection,
    util::add_box,
};
use crate::{
    csg::{self, Cube, Cylinder, Sphere},
    editor::util::add_csg,
    test_texture,
};

#[allow(clippy::too_many_arguments)]
pub fn editor_input_system(
    mut commands: Commands,

    mut offset: Local<Option<Vec3>>,

    keycodes: Res<Input<KeyCode>>,
    mut mouse_wheel: EventReader<MouseWheel>,

    mut selection: ResMut<Selection>,

    mut query: Query<&mut Brush>,
) {
    if keycodes.just_pressed(KeyCode::K) {
        let entity = commands
            .spawn()
            .insert(Brush::MinMax(Vec3::splat(-1.0), Vec3::splat(1.0)))
            .id();

        selection.primary = Some(entity);
    }

    if keycodes.just_pressed(KeyCode::L) {
        let entity = commands
            .spawn()
            .insert(Brush::Csg(Cube::new(Vec3::splat(2.0), 0.5).into()))
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
                    .insert(Brush::Csg(
                        Cylinder {
                            start: Vec3::new(0.0, -1.0, 0.0) + *offset,
                            end: Vec3::new(0.0, 1.0, 0.0) + *offset,

                            ..default()
                        }
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
                if let Brush::Csg(ref mut csg) = *brush {
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
                    Brush::MinMax(ref mut min, ref mut max) => {
                        *min += dmin;
                        *max += dmax;
                    }
                    Brush::Csg(ref mut csg) => {
                        csg.translate(dmin.into());
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

    mut query: Query<(Entity, &Brush), Changed<Brush>>,
    query_cleanup: Query<(&Handle<Mesh>, &Handle<StandardMaterial>)>,
) {
    for (entity, brush) in &mut query {
        // let entity = spawn_box(
        //     &mut commands,
        //     material,
        //     &mut meshes,
        //     Vec3::splat(-1.0),
        //     Vec3::splat(1.0),
        // );
        if let Ok((mesh, material)) = query_cleanup.get(entity) {
            info!("cleanup {:?} {:?}", mesh, material);
            meshes.remove(mesh);
            if let Some(material) = materials.remove(material) {
                if let Some(image) = material.base_color_texture {
                    info!("cleanup {:?}", image);
                    images.remove(image);
                }
            }
        }
        match brush {
            Brush::MinMax(min, max) => {
                let uv_test = images.add(Image::new(
                    Extent3d {
                        width: test_texture::TW as u32,
                        height: test_texture::TH as u32,
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    test_texture::create(),
                    TextureFormat::Rgba8Unorm,
                ));

                let material = materials.add(StandardMaterial {
                    base_color_texture: Some(uv_test),
                    metallic: 0.9,
                    perceptual_roughness: 0.1,
                    ..Default::default()
                });

                add_box(&mut commands, entity, material, &mut meshes, *min, *max);
            }
            Brush::Csg(csg) => {
                let material = materials.add(StandardMaterial {
                    base_color: Color::BLUE,
                    metallic: 0.9,
                    perceptual_roughness: 0.1,
                    ..Default::default()
                });

                add_csg(&mut commands, entity, material, &mut meshes, csg);
            }
        }
        info!("update brush mesh");
    }
}

#[allow(clippy::type_complexity)]
pub fn update_brush_csg_system(
    mut commands: Commands,

    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,

    query: Query<&Brush>,
    query_changed: Query<Entity, Changed<Brush>>,
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
            Brush::Csg(csg) => Some(csg),
            _ => None,
        })
        .collect::<Vec<_>>();

    let Some(mut u) = csgs.pop().cloned() else {
        info!( "no Csg brushes");
        return;
    };

    for csg in csgs {
        u = csg::union(&u, csg).unwrap();
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
