use std::f32::consts::{PI, TAU};

use super::{Csg, Polygon, Vertex};
use bevy::prelude::*;

pub struct Sphere {
    pub center: Vec3,
    pub r: f32,
    pub slices: usize,
    pub stacks: usize,
}

impl Default for Sphere {
    fn default() -> Self {
        Self {
            center: Vec3::ZERO,
            r: 0.5,
            slices: 16,
            stacks: 8,
        }
    }
}

impl Sphere {
    pub fn new(center: Vec3, r: f32, slices: usize, stacks: usize) -> Self {
        Self {
            center,
            r,
            slices,
            stacks,
        }
    }
}

impl From<Sphere> for Csg {
    fn from(sphere: Sphere) -> Self {
        let mut polygons = Vec::new();

        let vertex = |theta: f32, phi: f32| {
            let theta = theta * TAU;
            let phi = phi * PI;
            let dir = Vec3::new(theta.cos() * phi.sin(), phi.cos(), theta.sin() * phi.sin());
            //   vertices.push(Vertex::new(c + dir*r), dir);
            Vertex::new(sphere.center + dir * sphere.r, dir)
        };
        for i in 0..sphere.slices {
            for j in 0..sphere.stacks {
                let mut vertices = Vec::new();

                vertices.push(vertex(
                    i as f32 / sphere.slices as f32,
                    j as f32 / sphere.stacks as f32,
                ));

                if j > 0 {
                    vertices.push(vertex(
                        (i + 1) as f32 / sphere.slices as f32,
                        j as f32 / sphere.stacks as f32,
                    ));
                }
                if j < sphere.stacks - 1 {
                    vertices.push(vertex(
                        (i + 1) as f32 / sphere.slices as f32,
                        (j + 1) as f32 / sphere.stacks as f32,
                    ));
                }
                vertices.push(vertex(
                    i as f32 / sphere.slices as f32,
                    (j + 1) as f32 / sphere.stacks as f32,
                ));

                polygons.push(Polygon::from_vertices(vertices, 0));
            }
        }
        Csg::from_polygons(polygons)
    }
}
