use bevy::{
    pbr::wireframe::Wireframe,
    prelude::*,
    render::{mesh, view::RenderLayers},
};
use bevy_rapier3d::prelude::Collider;
use serde::{Deserialize, Serialize};

use crate::{
    csg::{self, Csg},
    render_layers,
};

use super::{
    components::{self, CsgOutput},
    resources,
};

pub fn spawn_box(
    commands: &mut Commands,
    material: Handle<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
    min: Vec3,
    max: Vec3,
) -> Entity {
    let center = (min + max) / 2.0;
    let hs = (max - min) / 2.0;

    commands
        .spawn(PbrBundle {
            mesh: meshes.add(
                mesh::shape::Box {
                    min_x: -hs.x,
                    max_x: hs.x,
                    min_y: -hs.y,
                    max_y: hs.y,
                    min_z: -hs.z,
                    max_z: hs.z,
                }
                .into(),
            ),
            material,
            transform: Transform::from_translation(center),
            ..Default::default()
        })
        .insert(Collider::cuboid(hs.x, hs.y, hs.z))
        .id()
}

pub fn add_box(
    commands: &mut Commands,
    entity: Entity,
    material: Handle<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
    min: Vec3,
    max: Vec3,
) {
    let center = (min + max) / 2.0;
    let hs = (max - min) / 2.0;

    commands
        .entity(entity)
        .insert(PbrBundle {
            mesh: meshes.add(
                mesh::shape::Box {
                    min_x: -hs.x,
                    max_x: hs.x,
                    min_y: -hs.y,
                    max_y: hs.y,
                    min_z: -hs.z,
                    max_z: hs.z,
                }
                .into(),
            ),
            material,
            transform: Transform::from_translation(center),
            ..Default::default()
        })
        .insert(Collider::cuboid(hs.x, hs.y, hs.z));
}

pub fn add_csg(
    commands: &mut Commands,
    entity: Entity,
    material: Handle<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
    csg: &Csg,
) {
    let center = Vec3::ZERO;

    commands
        .entity(entity)
        .insert(PbrBundle {
            mesh: meshes.add(csg.into()),
            material,
            transform: Transform::from_translation(center),
            ..Default::default()
        })
        // .insert(Collider::cuboid(hs.x, hs.y, hs.z))
        ;
}

pub fn spawn_csg_split(
    commands: &mut Commands,
    materials_res: &resources::Materials,
    meshes: &mut Assets<Mesh>,
    csg: &Csg,
) -> Vec<Entity> {
    let center = Vec3::ZERO;

    let split_meshes = csg::csg_to_split_meshes(csg);
    let mut entities = Vec::new();
    for (id, mesh) in split_meshes {
        let mesh = meshes.add(mesh);
        // todo some fallback if map lookups fail

        // let Some(material) = materials_res.get(material_name,materials, asset_server) else {
        //     warn!( "material resource not found for {}", material_name);
        //     continue;
        // };

        let mut entity_commands = commands.spawn((
            PbrBundle {
                mesh,
                // material,
                transform: Transform::from_translation(center),
                ..Default::default()
            },
            CsgOutput,
        ));

        if let Some(material_name) = materials_res.id_to_name_map.get(&id) {
            entity_commands.insert((
                components::MaterialRef {
                    material_name: material_name.clone(),
                },
                Name::new(format!("csg {:?}", material_name)),
                RenderLayers::layer(render_layers::MAIN_3D),
                Wireframe,
            ));
        } else {
            entity_commands.insert(Name::new("csg <no material>"));
        }
        debug!("spawned csg output: {:?}", entity_commands.id());
        entities.push(entity_commands.id());
    }
    entities
}

// // TODO: throw out with bevy 0.9
// #[derive(Debug, Clone, Copy)]
// pub struct Ray {
//     pub origin: Vec3,
//     pub direction: Vec3,
// }

// pub trait HackViewportToWorld {
//     fn viewport_to_world(
//         &self,
//         camera_transform: &GlobalTransform,
//         viewport_position: Vec2,
//     ) -> Option<Ray>;
// }

// impl HackViewportToWorld for Camera {
//     /// Returns a ray originating from the camera, that passes through everything beyond `viewport_position`.
//     ///
//     /// The resulting ray starts on the near plane of the camera.
//     ///
//     /// If the camera's projection is orthographic the direction of the ray is always equal to `camera_transform.forward()`.
//     ///
//     /// To get the world space coordinates with Normalized Device Coordinates, you should use
//     /// [`ndc_to_world`](Self::ndc_to_world).
//     fn viewport_to_world(
//         &self,
//         camera_transform: &GlobalTransform,
//         viewport_position: Vec2,
//     ) -> Option<Ray> {
//         let target_size = self.logical_viewport_size()?;
//         let ndc = viewport_position * 2. / target_size - Vec2::ONE;

//         let ndc_to_world = camera_transform.compute_matrix() * self.projection_matrix().inverse();
//         let world_near_plane = ndc_to_world.project_point3(ndc.extend(1.));
//         // Using EPSILON because an ndc with Z = 0 returns NaNs.
//         let world_far_plane = ndc_to_world.project_point3(ndc.extend(f32::EPSILON));

//         (!world_near_plane.is_nan() && !world_far_plane.is_nan()).then_some(Ray {
//             origin: world_near_plane,
//             direction: (world_far_plane - world_near_plane).normalize(),
//         })
//     }
// }

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
pub enum Orientation2d {
    DownFront,
    DownRight,

    Front,
    Right,
}

impl Default for Orientation2d {
    fn default() -> Self {
        Orientation2d::DownFront
    }
}

impl Orientation2d {
    pub fn flipped(&self) -> Orientation2d {
        match self {
            Orientation2d::DownFront => Orientation2d::DownRight,
            Orientation2d::DownRight => Orientation2d::DownFront,
            Orientation2d::Front => Orientation2d::Right,
            Orientation2d::Right => Orientation2d::Front,
        }
    }
    pub fn get_transform(&self) -> Transform {
        // TODO: check if this is all plausible...
        // TODO: better solution for near/far clipping in ortho projection(and where to put the camera...)
        const ORTHO_OFFSET: f32 = 100.0;
        match self {
            Orientation2d::DownFront => {
                Transform::from_xyz(0.0, ORTHO_OFFSET, 0.0).looking_at(Vec3::ZERO, Vec3::X)
            }
            Orientation2d::DownRight => {
                Transform::from_xyz(0.0, ORTHO_OFFSET, 0.0).looking_at(Vec3::ZERO, Vec3::Z)
            }
            Orientation2d::Front => {
                Transform::from_xyz(-ORTHO_OFFSET, 0.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y)
            }
            Orientation2d::Right => {
                Transform::from_xyz(0.0, 0.0, -ORTHO_OFFSET).looking_at(Vec3::ZERO, Vec3::Y)
            }
        }
    }
}

pub trait SnapToGrid {
    type Param;
    fn snap(self, s: Self::Param) -> Self;
}

impl SnapToGrid for f32 {
    type Param = f32;

    fn snap(self, s: Self::Param) -> Self {
        (self / s).round() * s
    }
}
