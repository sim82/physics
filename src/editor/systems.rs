use bevy::{
    input::mouse::MouseWheel,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use super::{components::Brush, resources::Selection, util::add_box};
use crate::test_texture;

#[allow(clippy::too_many_arguments)]
pub fn editor_input_system(
    mut commands: Commands,
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

        // let entity = spawn_box(
        //     &mut commands,
        //     material,
        //     &mut meshes,
        //     Vec3::splat(-1.0),
        //     Vec3::splat(1.0),
        // );

        match brush {
            Brush::MinMax(min, max) => {
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

                add_box(&mut commands, entity, material, &mut meshes, *min, *max);
            }
        }
        info!("update brush mesh");
    }
}
