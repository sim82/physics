use crate::csg::PLANE_EPSILON;

use super::{Csg, Location, Plane, Polygon, Vertex};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Component)]
pub struct Brush {
    pub planes: Vec<Plane>,
    pub appearances: Vec<i32>,
}

impl Brush {
    pub fn from_planes(planes: Vec<Plane>) -> Self {
        Brush {
            // appearances: std::iter::repeat(0).take(planes.len()).collect(),
            appearances: (0..planes.len() as i32).collect(),
            planes,
        }
    }

    /// get planes that are affected by a drag starting at this ray
    pub fn get_planes_behind_ray(&self, ray: Ray) -> Vec<(usize, f32)> {
        let mut res = Vec::new();
        for (i, p) in self.planes.iter().enumerate() {
            // info!("loc: {:?} {:?}", p.normal, location);
            let dot = p.normal.dot(ray.direction);
            debug!("dot: {}", dot);

            // check if face normal is orthogonal to ray
            // FIXME: it is probably too strict as soon as there are angled faces
            if dot.abs() > PLANE_EPSILON {
                continue;
            }
            let location = p.location_of_point(ray.origin);

            if location != Location::FRONT {
                continue;
            }

            res.push((i, p.w));
        }
        res
    }
}

impl Default for Brush {
    fn default() -> Self {
        let planes = vec![
            Plane::new(Vec3::X, 1.0),
            Plane::new(-Vec3::X, 1.0),
            Plane::new(Vec3::Y, 1.0),
            Plane::new(-Vec3::Y, 1.0),
            Plane::new(Vec3::Z, 1.0),
            Plane::new(-Vec3::Z, 1.0),
        ];
        Brush::from_planes(planes)
    }
}

#[derive(Debug)]
pub enum BrushError {
    Degenerated(Brush),
}

impl TryFrom<Brush> for Csg {
    type Error = BrushError;

    fn try_from(brush: Brush) -> Result<Self, Self::Error> {
        let mut polygons = Vec::new();
        for (i, (base_plane, appearance)) in brush
            .planes
            .iter()
            .zip(brush.appearances.iter())
            .enumerate()
        {
            const BASE_POLYGON_SIZE: f32 = 1024.0 * 8.0;
            let mut polygon = create_base_polygon(base_plane, *appearance, BASE_POLYGON_SIZE);

            for (j, plane) in brush.planes.iter().enumerate() {
                if i == j {
                    continue;
                }
                let mut coplanar_front = Vec::new();
                let mut front = Vec::new();

                let mut coplanar_back = Vec::new();
                let mut back = Vec::new();

                plane.split_polygon(
                    &polygon,
                    &mut coplanar_front,
                    &mut coplanar_back,
                    &mut front,
                    &mut back,
                );
                // println!(
                //     "{} {} {} {}",
                //     coplanar_front.len(),
                //     front.len(),
                //     coplanar_back.len(),
                //     back.len()
                // );

                // check degenerated cases. the split polygon must either be cut in two or must be completely behind the plane
                // coplanar or no back result would mean the planes describe an empty volume

                if !(coplanar_back.is_empty() && coplanar_front.is_empty())
                    || back.len() != 1
                    || front.len() > 1
                {
                    // TODO: include more info about degenrated case?
                    return Err(BrushError::Degenerated(brush));
                }
                assert!(coplanar_back.is_empty());
                assert!(coplanar_front.is_empty());
                assert!(back.len() == 1);
                assert!(front.len() <= 1);

                polygon = back.pop().unwrap()
            }
            polygons.push(polygon);
        }
        Ok(Csg::from_polygons(polygons))
    }
}

fn create_base_polygon(plane: &Plane, appearance: i32, width: f32) -> Polygon {
    let normal = plane.normal.normalize();
    let (x, y) = normal.any_orthonormal_pair();
    let origin = normal * plane.w;

    Polygon::from_vertices(
        vec![
            Vertex::new(origin + x * width, normal),
            Vertex::new(origin + y * width, normal),
            Vertex::new(origin - x * width, normal),
            Vertex::new(origin - y * width, normal),
        ],
        appearance,
    )
}

#[test]
fn test_create_base_polygon() {
    let polygon = create_base_polygon(&Plane::new(Vec3::X, 1.0), 0, 16.0);
    println!("polygon: {:?}", polygon);
    let polygon = create_base_polygon(&Plane::new(-Vec3::X, 1.0), 0, 16.0);
    println!("polygon: {:?}", polygon);
}

#[test]
fn test_csg_from_brush() {
    let brush = Brush::default();
    let csg: Csg = brush.try_into().unwrap();

    println!("{:?}", csg);
}
