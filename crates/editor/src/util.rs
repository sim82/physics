use super::components::{self, CsgOutput};
use shared::render_layers;

use bevy::{
    pbr::wireframe::Wireframe,
    prelude::*,
    render::{mesh, view::RenderLayers},
};
use bevy_rapier3d::prelude::Collider;
use csg::{self, Csg};
use serde::{Deserialize, Serialize};

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
    let (mesh, origin) = csg.into();
    commands
        .entity(entity)
        .insert(PbrBundle {
            mesh: meshes.add(mesh),
            material,
            transform: Transform::from_translation(origin),
            ..Default::default()
        })
        // .insert(Collider::cuboid(hs.x, hs.y, hs.z))
        ;
}
pub fn spawn_csg_split(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    csg: &Csg,
    origin: Vec3,
    material_names: &[String],
) -> Vec<Entity> {
    // de-duplicate generated meshes by material id:

    // generate list of unique material ids (each polyon in the csg can have different appearance ids, but they can all be mapped to the same material name)
    let mut unique_materials = material_names.iter().collect::<Vec<_>>();
    unique_materials.sort();
    unique_materials.dedup();

    // create one vector per material to collect generated triangles, using ref-cell to allow mutliple mutable aliases per vector
    let mut ref_cells = Vec::new();
    for _ in &unique_materials {
        ref_cells.push(std::cell::RefCell::new(Vec::new()));
    }

    // create a vector with an entry for each *appearance id* pointing to the ouput vector (note: multiple entries can refenrence the same vector!)
    let mut output = Vec::new();
    for name in material_names {
        let unique_index = unique_materials.binary_search(&name).unwrap();
        output.push(&ref_cells[unique_index]);
    }

    // generate trianles that get put into the 'per appearance' vectors (which actually are backed by 'per materia' vectors using RefCell)
    csg::csg_to_split_tri_lists(csg, &output[..]);
    let mut entities = Vec::new();

    // generate one mesh per material
    // TODO: maybe do more optimizations like vertex merging and proper t-junctions etc... probably not useful without further merging meshes
    for (i, tri_list) in ref_cells.drain(..).enumerate() {
        let material_name = unique_materials[i];

        let mut tri_list = tri_list.into_inner();
        if tri_list.is_empty() {
            // this can happen when all faces of a certain material are clipped away by csg
            // warn!("empty tri list for material: {}", material_name);
            continue;
        }

        for tri in &mut tri_list {
            for v in &mut tri.0 {
                *v -= origin;
            }
        }

        // FIXME: this is crap: we don't really need to create an entity here
        let mesh = if material_name != "material/special/sky1" {
            let texgen = csg::texgen::Texgen::with_offset(origin);
            let mesh = csg::triangles_to_mesh_with_texgen(&tri_list, &texgen);
            meshes.add(mesh)
        } else {
            default()
        };

        let mut entity_commands = commands.spawn((
            PbrBundle {
                mesh,
                ..Default::default()
            },
            CsgOutput,
        ));

        // if let Some(material_name) = materials_res.id_to_name_map.get(&id) {
        entity_commands.insert((
            components::MaterialRef {
                material_name: material_name.clone(),
            },
            Name::new(format!("csg {:?}", material_name)),
            RenderLayers::layer(render_layers::MAIN_3D),
            Wireframe,
            Visibility::Hidden, // do not render with default material to prevent seizure...
        ));
        debug!("spawned csg output: {:?}", entity_commands.id());
        entities.push(entity_commands.id());
    }
    entities
}

// pub fn spawn_csg_split_old(
//     commands: &mut Commands,
//     materials_res: &resources::Materials,
//     meshes: &mut Assets<Mesh>,
//     csg: &Csg,
//     origin: Vec3,
//     material_names: &[String],
// ) -> Vec<Entity> {
//     // let center = Vec3::ZERO;

//     let split_meshes = csg::csg_to_split_meshes_relative_to_origin(csg, origin);
//     let mut entities = Vec::new();
//     for (id, mesh) in split_meshes {
//         let mesh = meshes.add(mesh);
//         // todo some fallback if map lookups fail

//         // let Some(material) = materials_res.get(material_name,materials, asset_server) else {
//         //     warn!( "material resource not found for {}", material_name);
//         //     continue;
//         // };

//         let mut entity_commands = commands.spawn((
//             PbrBundle {
//                 mesh,
//                 // material,
//                 // transform: Transform::from_translation(center),
//                 ..Default::default()
//             },
//             CsgOutput,
//         ));

//         // if let Some(material_name) = materials_res.id_to_name_map.get(&id) {
//         if let Some(material_name) = material_names.get(id as usize) {
//             entity_commands.insert((
//                 components::MaterialRef {
//                     material_name: material_name.clone(),
//                 },
//                 Name::new(format!("csg {:?}", material_name)),
//                 RenderLayers::layer(render_layers::MAIN_3D),
//                 Wireframe,
//             ));
//         } else {
//             entity_commands.insert(Name::new("csg <no material>"));
//         }
//         debug!("spawned csg output: {:?}", entity_commands.id());
//         entities.push(entity_commands.id());
//     }
//     entities
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
                Transform::from_xyz(0.0, ORTHO_OFFSET, 0.0).looking_at(Vec3::ZERO, -Vec3::Z)
            }
            Orientation2d::DownRight => {
                Transform::from_xyz(0.0, ORTHO_OFFSET, 0.0).looking_at(Vec3::ZERO, -Vec3::X)
            }
            Orientation2d::Front => {
                Transform::from_xyz(0.0, 0.0, ORTHO_OFFSET).looking_at(Vec3::ZERO, Vec3::Y)
            }
            Orientation2d::Right => {
                Transform::from_xyz(ORTHO_OFFSET, 0.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y)
            }
        }
    }
    pub fn get_up_axis(&self, v: Vec3) -> f32 {
        match self {
            Orientation2d::DownFront => v.z,
            Orientation2d::DownRight => v.x,
            Orientation2d::Front => v.y,
            Orientation2d::Right => v.y,
        }
    }
    pub fn get_up_axis_mut<'a>(&self, v: &'a mut Vec3) -> &'a mut f32 {
        match self {
            Orientation2d::DownFront => &mut v.z,
            Orientation2d::DownRight => &mut v.x,
            Orientation2d::Front => &mut v.y,
            Orientation2d::Right => &mut v.y,
        }
    }
    pub fn get_right_axis(&self, v: Vec3) -> f32 {
        match self {
            Orientation2d::DownFront => v.x,
            Orientation2d::DownRight => v.z,
            Orientation2d::Front => v.x,
            Orientation2d::Right => v.z,
        }
    }
    pub fn get_right_axis_mut<'a>(&self, v: &'a mut Vec3) -> &'a mut f32 {
        match self {
            Orientation2d::DownFront => &mut v.x,
            Orientation2d::DownRight => &mut v.z,
            Orientation2d::Front => &mut v.x,
            Orientation2d::Right => &mut v.z,
        }
    }

    pub fn mix(&self, origin: Vec3, cursor: Vec3) -> Vec3 {
        match self {
            Orientation2d::DownFront | Orientation2d::DownRight => {
                Vec3::new(origin.x, cursor.y, origin.z)
            }
            Orientation2d::Front => Vec3::new(origin.x, origin.y, cursor.z),
            Orientation2d::Right => Vec3::new(cursor.x, origin.y, origin.z),
        }
    }

    pub fn get_grid_rotation(&self) -> Quat {
        match self {
            Orientation2d::DownRight | Orientation2d::DownFront => Quat::IDENTITY,
            Orientation2d::Front => Quat::from_axis_angle(Vec3::X, 90_f32.to_radians()),
            Orientation2d::Right => Quat::from_axis_angle(Vec3::Z, 90_f32.to_radians()),
        }
    }
    pub fn get_lower_x_axis_color(&self) -> Color {
        match self {
            Orientation2d::Front => Color::rgb(1.0, 0.2, 0.2),
            Orientation2d::Right => Color::rgb(0.0, 1.0, 0.2),
            _ => Color::PINK,
        }
    }
    pub fn get_lower_z_axis_color(&self) -> Color {
        match self {
            Orientation2d::Front => Color::rgb(0.2, 1.0, 0.2),
            Orientation2d::Right => Color::rgb(0.2, 0.2, 1.0),
            _ => Color::PINK,
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

impl SnapToGrid for Vec3 {
    type Param = f32;
    fn snap(self, s: Self::Param) -> Vec3 {
        Vec3::new(self.x.snap(s), self.y.snap(s), self.z.snap(s))
    }
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub enum WmMouseButton {
    #[default]
    Left,
    Middle,
    Right,
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub struct WmModifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

#[derive(Debug, Default, Copy, Clone)]
pub struct WmEventPointerState {
    pub pos: Vec2,
    pub bounds: Rect,
    pub modifiers: WmModifiers,
}

impl WmEventPointerState {
    pub fn get_pos_origin_down(&self) -> Vec2 {
        // flip y coord
        Vec2::new(
            self.pos.x - self.bounds.min.x,
            self.bounds.max.y - self.pos.y,
        )
    }
}

#[derive(Debug)]
pub enum WmEvent {
    Clicked {
        window: &'static str,
        button: WmMouseButton,
        pointer_state: WmEventPointerState,
    },
    DragStart {
        window: &'static str,
        button: WmMouseButton,
        pointer_state: WmEventPointerState,
    },
    DragUpdate {
        window: &'static str,
        button: WmMouseButton,
        pointer_state: WmEventPointerState,
    },
    DragEnd {
        window: &'static str,
        button: WmMouseButton,
        pointer_state: WmEventPointerState,
    },
    ZoomDelta(f32),
}

// https://mathworld.wolfram.com/Point-LineDistance3-Dimensional.html
pub fn ray_point_distance(ray: Ray, x0: Vec3) -> f32 {
    let x1 = ray.origin;
    let x2 = ray.origin + ray.direction;
    (x0 - x1).cross(x0 - x2).length() / ray.direction.length()
}

pub trait TriangleTrait {
    fn v0(&self) -> Vec3;
    fn v1(&self) -> Vec3;
    fn v2(&self) -> Vec3;
}

impl TriangleTrait for [Vec3; 3] {
    fn v0(&self) -> Vec3 {
        self[0]
    }

    fn v1(&self) -> Vec3 {
        self[1]
    }

    fn v2(&self) -> Vec3 {
        self[2]
    }
}

#[derive(Default, Debug)]
pub struct RayHit {
    pub distance: f32,
    pub uv_coords: (f32, f32),
}

/// Implementation of the Möller-Trumbore ray-triangle intersection test
/// adapted from https://github.com/aevyrie/bevy_mod_raycast/blob/main/src/raycast.rs
pub fn raycast_moller_trumbore(
    ray: &Ray,
    triangle: &impl TriangleTrait,
    cull_backfaces: bool,
) -> Option<RayHit> {
    // Source: https://www.scratchapixel.com/lessons/3d-basic-rendering/ray-tracing-rendering-a-triangle/moller-trumbore-ray-triangle-intersection
    let vector_v0_to_v1 = triangle.v1() - triangle.v0();
    let vector_v0_to_v2 = triangle.v2() - triangle.v0();
    let p_vec = ray.direction.cross(vector_v0_to_v2);
    let determinant: f32 = vector_v0_to_v1.dot(p_vec);

    if (cull_backfaces && determinant < std::f32::EPSILON)
        || (!cull_backfaces && determinant.abs() < std::f32::EPSILON)
    {
        // if determinant.abs() < std::f32::EPSILON {
        return None;
    }

    let determinant_inverse = 1.0 / determinant;

    let t_vec = ray.origin - triangle.v0();
    let u = t_vec.dot(p_vec) * determinant_inverse;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }

    let q_vec = t_vec.cross(vector_v0_to_v1);
    let v = ray.direction.dot(q_vec) * determinant_inverse;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }

    // The distance between ray origin and intersection is t.
    let t: f32 = vector_v0_to_v2.dot(q_vec) * determinant_inverse;

    Some(RayHit {
        distance: t,
        uv_coords: (u, v),
    })
}
