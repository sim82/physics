use super::{Csg, Polygon, Vertex};
use bevy::prelude::*;
use std::f32::consts::TAU;
// Construct a solid cylinder. Optional parameters are `start`, `end`,
// `radius`, and `slices`, which default to `[0, -1, 0]`, `[0, 1, 0]`, `1`, and
// `16`. The `slices` parameter controls the tessellation.
//
// Example usage:
//
//     var cylinder = CSG.cylinder({
//       start: [0, -1, 0],
//       end: [0, 1, 0],
//       radius: 1,
//       slices: 16
//     });

pub struct Cylinder {
    pub start: Vec3,
    pub end: Vec3,
    pub radius: f32,
    pub slices: usize,
}

impl Default for Cylinder {
    fn default() -> Self {
        Self {
            start: -Vec3::Y,
            end: Vec3::Y,
            radius: 1.0,
            slices: 16,
        }
    }
}
impl Cylinder {
    pub fn new(start: Vec3, end: Vec3, radius: f32, slices: usize) -> Cylinder {
        Cylinder {
            start,
            end,
            radius,
            slices,
        }
    }
}

impl From<Cylinder> for Csg {
    fn from(cylinder: Cylinder) -> Self {
        let ray = cylinder.end - cylinder.start;
        let axis_z = ray.normalize();
        let (is_y, not_is_y) = if axis_z.y > 0.5 {
            (1.0, 0.0)
        } else {
            (0.0, 1.0)
        };
        let axis_x = Vec3::new(is_y, not_is_y, 0.0).cross(axis_z).normalize();
        let axis_y = axis_x.cross(axis_z).normalize();
        let start = Vertex::new(cylinder.start, -axis_z);
        let end = Vertex::new(cylinder.end, axis_z.normalize());
        let mut polygons = Vec::new();
        let point = |stack: f32, slice: f32, normal_blend: f32| {
            let angle = slice * TAU;
            let out = axis_x * angle.cos() + axis_y * angle.sin();
            let pos = cylinder.start + ray * stack + out * cylinder.radius;
            let normal = out * (1.0 - normal_blend.abs()) + axis_z * normal_blend;
            Vertex::new(pos, normal)
        };
        for i in 0..cylinder.slices {
            let t0 = i as f32 / cylinder.slices as f32;
            let t1 = (i + 1) as f32 / cylinder.slices as f32;
            polygons.push(Polygon::from_vertices(vec![
                start,
                point(0.0, t0, -1.0),
                point(0.0, t1, -1.0),
            ]));
            polygons.push(Polygon::from_vertices(vec![
                point(0.0, t1, 0.0),
                point(0.0, t0, 0.0),
                point(1.0, t0, 0.0),
                point(1.0, t1, 0.0),
            ]));
            polygons.push(Polygon::from_vertices(vec![
                end,
                point(1.0, t1, 1.0),
                point(1.0, t0, 1.0),
            ]));
        }
        Csg::from_polygons(polygons)
    }
}
