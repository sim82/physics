use bevy::prelude::*;

use super::{Csg, Plane, Polygon, Vertex};

#[derive(Debug, Clone)]
pub struct Brush {
    pub planes: Vec<Plane>,
}

impl Brush {
    pub fn from_planes(planes: Vec<Plane>) -> Self {
        Brush { planes }
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

impl From<Brush> for Csg {
    fn from(brush: Brush) -> Self {
        let mut polygons = Vec::new();
        for (i, base_plane) in brush.planes.iter().enumerate() {
            let mut polygon = create_base_polygon(base_plane, 128.0);

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
                assert!(coplanar_back.is_empty());
                assert!(coplanar_front.is_empty());
                assert!(back.len() == 1);
                assert!(front.len() <= 1);

                polygon = back.pop().unwrap()
            }
            polygons.push(polygon);
        }
        Csg::from_polygons(polygons)
    }
}

fn create_base_polygon(plane: &Plane, width: f32) -> Polygon {
    let normal = plane.normal.normalize();
    let (x, y) = normal.any_orthonormal_pair();
    let origin = normal * plane.w;

    Polygon::from_vertices(vec![
        Vertex::new(origin + x * width, normal),
        Vertex::new(origin + y * width, normal),
        Vertex::new(origin - x * width, normal),
        Vertex::new(origin - y * width, normal),
    ])
}

#[test]
fn test_create_base_polygon() {
    let polygon = create_base_polygon(&Plane::new(Vec3::X, 1.0), 16.0);
    println!("polygon: {:?}", polygon);
    let polygon = create_base_polygon(&Plane::new(-Vec3::X, 1.0), 16.0);
    println!("polygon: {:?}", polygon);
}

#[test]
fn test_csg_from_brush() {
    let brush = Brush::default();
    let csg: Csg = brush.into();

    println!("{:?}", csg);
}
