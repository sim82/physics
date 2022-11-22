use super::{Csg, Polygon, Vertex};
use bevy::prelude::*;

pub struct Cube {
    c: Vec3,
    r: f32,
}

impl Default for Cube {
    fn default() -> Self {
        Self {
            c: Vec3::ZERO,
            r: 1.0,
        }
    }
}

impl Cube {
    pub fn new(c: Vec3, r: f32) -> Self {
        Cube { c, r }
    }
}

impl From<Cube> for Csg {
    fn from(cube: Cube) -> Csg {
        let info = [
            ([0, 4, 6, 2], (-1, 0, 0)),
            ([1, 3, 7, 5], (1, 0, 0)),
            ([0, 1, 5, 4], (0, -1, 0)),
            ([2, 6, 7, 3], (0, 1, 0)),
            ([0, 2, 3, 1], (0, 0, -1)),
            ([4, 5, 7, 6], (0, 0, 1)),
        ];
        let c = cube.c;
        let r = Vec3::splat(cube.r);
        let polygons = info
            .iter()
            .map(|(points, normal)| {
                let vtx = points
                    .iter()
                    .map(|i| {
                        Vertex::new(
                            c + r * Vec3::new(
                                if (i & 1) != 0 { 2.0 } else { 0.0 } - 1.0,
                                if (i & 2) != 0 { 2.0 } else { 0.0 } - 1.0,
                                if (i & 4) != 0 { 2.0 } else { 0.0 } - 1.0,
                            ),
                            Vec3::new(normal.0 as f32, normal.1 as f32, normal.2 as f32),
                        )
                    })
                    .collect();
                Polygon::from_vertices(vtx, 0)
            })
            .collect();

        Csg::from_polygons(polygons)
    }
}
