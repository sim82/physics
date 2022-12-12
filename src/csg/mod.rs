// Constructive Solid Geometry (CSG) is a modeling technique that uses Boolean
// operations like union and intersection to combine 3D solids. This library
// implements CSG operations on meshes elegantly and concisely using BSP trees,
// and is meant to serve as an easily understandable implementation of the
// algorithm. All edge cases involving overlapping coplanar polygons in both
// solids are correctly handled.
//
// Example usage:
//
//     let cube :Csg = Cube::default().into();
//     var sphere :Csg = Sphere{ radius: 1.3 }.into();
//     var triangles = subtract(&cube, &sphere).all_triangles();
//
// ## Implementation Details
//
// All CSG operations are implemented in terms of two functions, `clip_to()` and
// `invert()`, which remove parts of a BSP tree inside another BSP tree and swap
// solid and empty space, respectively. To find the union of `a` and `b`, we
// want to remove everything in `a` inside `b` and everything in `b` inside `a`,
// then combine polygons from `a` and `b` into one solid:
//
//     a.clip_to(b);
//     b.clip_to(a);
//     a.build(b.all_polygons());
//
// The only tricky part is handling overlapping coplanar polygons in both trees.
// The code above keeps both copies, but we need to keep them in one tree and
// remove them in the other tree. To remove them from `b` we can clip the
// inverse of `b` against `a`. The code for union now looks like this:
//
//     a.clip_to(b);
//     b.clip_to(a);
//     b.invert();
//     b.clip_to(a);
//     b.invert();
//     a.build(b.all_polygons());
//
// Subtraction and intersection naturally follow from set operations. If
// union is `A | B`, subtraction is `A - B = ~(~A | B)` and intersection is
// `A & B = ~(~A | ~B)` where `~` is the complement operator.
//
// Original code and comments copyright (c) 2011 Evan Wallace (http://madebyevan.com/), under the MIT license.

use bevy::{
    prelude::*,
    render::{mesh::Indices, primitives::Aabb, render_resource::PrimitiveTopology},
    utils::HashMap,
};

// use bevy_rapier3d::rapier::geometry::Collider;

mod cube;
use bevy_rapier3d::prelude::Collider;
pub use cube::Cube;

mod cylinder;
pub use cylinder::Cylinder;

mod sphere;
use serde::{Deserialize, Serialize};
pub use sphere::Sphere;

mod brush;
pub use brush::Brush;

mod texgen;
use self::texgen::Texgen;

// clean slate, bevy flavoured, port of csg.js

#[derive(Clone, Copy, Default, Debug, Serialize, Deserialize, bevy_inspector_egui::Inspectable)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
}

impl Vertex {
    pub fn new(position: Vec3, normal: Vec3) -> Self {
        Vertex { position, normal }
    }

    // Invert all orientation-specific data (e.g. vertex normal). Called when the
    // orientation of a polygon is flipped.
    pub fn flip(&mut self) {
        self.normal = -self.normal;
    }

    pub fn flipped(&self) -> Vertex {
        Vertex {
            position: self.position,
            normal: -self.normal,
        }
    }
    // Create a new vertex between this vertex and `other` by linearly
    // interpolating all properties using a parameter of `t`. Subclasses should
    // override this to interpolate additional properties.
    pub fn interpolated(&self, other: &Vertex, f: f32) -> Self {
        Self {
            position: self.position.lerp(other.position, f),
            normal: self.normal.lerp(other.normal, f),
        }
    }
}

// `CSG.Plane.EPSILON` is the tolerance used by `splitPolygon()` to decide if a
// point is on the plane.
pub const PLANE_EPSILON: f32 = 1e-3; // HACK: work around stability problems. not sure if this is the right approach

#[derive(Clone, Debug, Default, Copy, Serialize, Deserialize, bevy_inspector_egui::Inspectable)]
pub struct Plane {
    pub normal: Vec3,
    pub w: f32,
}

bitflags::bitflags! {
    pub struct Location : u32 {
        const NONE = 0;
        const COPLANAR = Self::NONE.bits();
        const FRONT = 1;
        const BACK = 2;
        const SPANNING = Self::FRONT.bits() | Self::BACK.bits;
    }
}

pub struct SplitPolygonsResult {
    coplanar_front: Vec<Polygon>,
    coplanar_back: Vec<Polygon>,
    front: Vec<Polygon>,
    back: Vec<Polygon>,
}

impl SplitPolygonsResult {
    pub fn into_merged(mut self) -> (Vec<Polygon>, Vec<Polygon>) {
        self.coplanar_front.append(&mut self.front);
        self.coplanar_back.append(&mut self.back);
        (self.coplanar_front, self.coplanar_back)
    }
}

impl Plane {
    pub fn new(normal: Vec3, w: f32) -> Self {
        Plane { normal, w }
    }
    pub fn from_points(a: Vec3, b: Vec3, c: Vec3) -> Self {
        let normal = (b - a).cross(c - a).normalize_or_zero(); // TODO: error handling for degenerated cases?
        Plane {
            normal,
            w: normal.dot(a),
        }
    }
    pub fn from_points_slice(s: &[Vec3; 3]) -> Self {
        Self::from_points(s[0], s[1], s[2])
    }
    pub fn flip(&mut self) {
        self.normal = -self.normal;
        self.w = -self.w;
    }
    pub fn flipped(&self) -> Plane {
        Plane {
            normal: -self.normal,
            w: -self.w,
        }
    }

    // Split `polygon` by this plane if needed, then put the polygon or polygon
    // fragments in the appropriate lists. Coplanar polygons go into either
    // `coplanarFront` or `coplanarBack` depending on their orientation with
    // respect to this plane. Polygons in front or in back of this plane go into
    // either `front` or `back`.
    pub fn split_polygon(
        &self,
        polygon: &Polygon,
        coplanar_front: &mut Vec<Polygon>,
        coplanar_back: &mut Vec<Polygon>,
        front: &mut Vec<Polygon>,
        back: &mut Vec<Polygon>,
    ) {
        // Classify each point as well as the entire polygon into one of the above
        // four classes.
        let mut polygon_type = Location::NONE;
        let mut types = Vec::new();

        for v in &polygon.vertices {
            let vertex_pos = v.position;
            let location = self.location_of_point(vertex_pos);

            polygon_type |= location;
            types.push(location);
        }

        // Put the polygon in the correct list, splitting it when necessary.
        match polygon_type {
            Location::COPLANAR if self.normal.dot(polygon.plane.normal) > 0.0 => {
                coplanar_front.push(polygon.clone())
            }
            Location::COPLANAR => coplanar_back.push(polygon.clone()),
            Location::FRONT => front.push(polygon.clone()),
            Location::BACK => back.push(polygon.clone()),

            Location::SPANNING => {
                // var f = [], b = [];
                let mut f = Vec::new();
                let mut b = Vec::new();
                for (i, vi) in polygon.vertices.iter().enumerate() {
                    let j = (i + 1) % polygon.vertices.len();
                    let ti = types[i];
                    let tj = types[j];
                    let vj = &polygon.vertices[j];

                    if ti != Location::BACK {
                        f.push(*vi);
                    }
                    if ti != Location::FRONT {
                        b.push(*vi)
                    }
                    if (ti | tj) == Location::SPANNING {
                        let t = (self.w - self.normal.dot(vi.position))
                            / self.normal.dot(vj.position - vi.position);
                        let v = vi.interpolated(vj, t);
                        f.push(v);
                        b.push(v);
                    }
                }
                if f.len() >= 3 {
                    front.push(Polygon::from_vertices(f, polygon.a))
                }

                if b.len() >= 3 {
                    back.push(Polygon::from_vertices(b, polygon.a))
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn location_of_point(&self, vertex_pos: Vec3) -> Location {
        let t = self.normal.dot(vertex_pos) - self.w;
        if t < -PLANE_EPSILON {
            Location::BACK
        } else if t > PLANE_EPSILON {
            Location::FRONT
        } else {
            Location::COPLANAR
        }
    }

    pub fn split_polygons(&self, polygons: &[Polygon]) -> SplitPolygonsResult {
        let mut front = Vec::new();
        let mut back = Vec::new();
        let mut coplanar_front = Vec::new();
        let mut coplanar_back = Vec::new();

        for polygon in polygons {
            self.split_polygon(
                polygon,
                &mut coplanar_front,
                &mut coplanar_back,
                &mut front,
                &mut back,
            )
        }
        SplitPolygonsResult {
            coplanar_front,
            coplanar_back,
            front,
            back,
        }
    }
}

// Represents a convex polygon. The vertices used to initialize a polygon must
// be coplanar and form a convex loop.
//
// TODO: Each convex polygon has a `shared` property, which is shared between all
// polygons that are clones of each other or were split from the same polygon.
// This can be used to define per-polygon properties (such as surface color).
#[derive(Clone, Debug, Default, Serialize, Deserialize, bevy_inspector_egui::Inspectable)]
pub struct Polygon {
    pub vertices: Vec<Vertex>,
    pub plane: Plane,
    pub a: i32,
}

impl Polygon {
    fn plane_from_vertices(vs: &[Vertex]) -> Plane {
        assert!(vs.len() >= 3);
        Plane::from_points(vs[0].position, vs[1].position, vs[2].position)
    }

    pub fn flip(&mut self) {
        self.vertices.reverse();
        for v in &mut self.vertices {
            v.flip();
        }
        self.plane.flip();
    }

    pub fn flipped(&self) -> Polygon {
        let mut vertices = self
            .vertices
            .iter()
            .map(Vertex::flipped)
            .collect::<Vec<_>>();
        vertices.reverse();
        Polygon {
            vertices,
            plane: self.plane.flipped(),
            a: self.a,
        }
    }
    pub fn from_vertices(vertices: Vec<Vertex>, a: i32) -> Polygon {
        Polygon {
            plane: Polygon::plane_from_vertices(&vertices[0..3]),
            vertices,
            a,
        }
    }
    pub fn translate(&mut self, offset: Vec3) {
        assert!(self.vertices.len() >= 3);
        for v in &mut self.vertices {
            v.position += offset;
        }
        self.plane = Polygon::plane_from_vertices(&self.vertices[0..3]);
    }
}

// Holds a binary space partition tree representing a 3D solid. Two solids can
// be combined using the `union()`, `subtract()`, and `intersect()` functions.
#[derive(Clone, Debug, Default, Serialize, Deserialize, bevy_inspector_egui::Inspectable)]
pub struct Csg {
    pub polygons: Vec<Polygon>,
}

impl Csg {
    pub fn from_polygons(polygons: Vec<Polygon>) -> Self {
        Csg { polygons }
    }

    pub fn get_triangles_no_appearance(&self) -> Vec<([Vec3; 3], Vec3)> {
        let mut res = Vec::new();

        for p in &self.polygons {
            if p.vertices.len() < 3 {
                continue;
            }
            // premature and completely unnecessary optimization
            res.reserve(p.vertices.len() - 2);

            // crate triangle 'fans':
            // all triangles share 1st point
            let v0 = p.vertices[0];
            // sweep over 2-windows of the remaining vertices to get 2nd and 3rd points
            for vs in p.vertices[1..].windows(2) {
                res.push((
                    [v0.position, vs[0].position, vs[1].position],
                    p.plane.normal,
                ));
            }
        }
        res
    }

    pub fn get_triangles(&self) -> Vec<([Vec3; 3], Vec3, i32)> {
        let mut res = Vec::new();

        for p in &self.polygons {
            if p.vertices.len() < 3 {
                continue;
            }
            // premature and completely unnecessary optimization
            res.reserve(p.vertices.len() - 2);

            // crate triangle 'fans':
            // all triangles share 1st point
            let v0 = p.vertices[0];
            // sweep over 2-windows of the remaining vertices to get 2nd and 3rd points
            for vs in p.vertices[1..].windows(2) {
                res.push((
                    [v0.position, vs[0].position, vs[1].position],
                    p.plane.normal,
                    p.a,
                ));
            }
        }
        res
    }

    pub fn get_collision_polygons(&self) -> Vec<(Collider, Vec3)> {
        let mut colliders = Vec::new();
        for p in &self.polygons {
            let points = p.vertices.iter().map(|v| v.position).collect::<Vec<_>>();
            // let origin = points[0];
            // for p in &mut points {
            //     *p -= origin;
            // }
            let origin = Vec3::ZERO;
            let mut indices = Vec::new();
            // sweep over 2-windows of the remaining vertices to get 2nd and 3rd points
            for vs in 1..(points.len() as u32 - 1) {
                indices.push([0, vs, vs + 1]);
            }
            info!("points: {:?}, indices: {:?}", points, indices);
            // colliders.push(Collider::convex_mesh(points, &indices[..]).unwrap());
            colliders.push((Collider::trimesh(points, indices), origin));
        }
        colliders
    }
    pub fn invert(&mut self) {
        for p in &mut self.polygons {
            p.flip();
        }
    }

    pub fn inverted(&self) -> Csg {
        Csg::from_polygons(self.polygons.iter().map(Polygon::flipped).collect())
    }
    pub fn translate(&mut self, offset: Vec3) {
        for p in &mut self.polygons {
            p.translate(offset);
        }
    }
    pub fn get_aabb(&self) -> Aabb {
        let mut min = Vec3::splat(1e10);
        let mut max = Vec3::splat(-1e10);
        for polygon in &self.polygons {
            for Vertex {
                position,
                normal: _,
            } in &polygon.vertices
            {
                min = min.min(*position);
                max = max.max(*position);
            }
        }
        Aabb::from_min_max(min, max)
    }

    pub fn bounding_sphere(&self) -> (Vec3, f32) {
        let num: f32 = self
            .polygons
            .iter()
            .map(|p| p.vertices.len())
            .sum::<usize>() as f32;
        let center: Vec3 = self
            .polygons
            .iter()
            .flat_map(|p| p.vertices.iter().map(|v| v.position / num))
            .sum();
        let radius = self
            .polygons
            .iter()
            .flat_map(|p| {
                p.vertices
                    .iter()
                    .map(|v| bevy::utils::FloatOrd((center - v.position).length()))
            })
            .max()
            .unwrap()
            .0;

        (center, radius)
    }
}

// Holds a node in a BSP tree. A BSP tree is built from a collection of polygons
// by picking a polygon to split along. That polygon (and all other coplanar
// polygons) are added directly to that node and the other polygons are added to
// the front and/or back subtrees. This is not a leafy BSP tree since there is
// no distinction between internal and leaf nodes.
#[derive(Clone, Debug, Default)]
pub struct Node {
    pub plane: Plane,
    pub front: Option<Box<Node>>,
    pub back: Option<Box<Node>>,
    pub polygons: Vec<Polygon>,
}

impl Node {
    // Build a BSP tree out of `polygons`.
    // Each set of polygons is partitioned using the first polygon
    // (no heuristic is used to pick a good split).
    pub fn from_polygons(polygons: &[Polygon]) -> Option<Node> {
        if polygons.is_empty() {
            return None;
        }

        let plane = polygons[0].plane;
        let SplitPolygonsResult {
            coplanar_front: mut polygons,
            mut coplanar_back,
            front,
            back,
        } = plane.split_polygons(polygons);

        polygons.append(&mut coplanar_back);

        let front = Node::from_polygons(&front).map(Box::new);
        let back = Node::from_polygons(&back).map(Box::new);

        Some(Node {
            plane,
            front,
            back,
            polygons,
        })
    }

    // Insert polygons into existing tree. The new polygons are filtered down to the bottom
    // of the tree and become new nodes there. Each set of polygons is partitioned using the
    // first polygon (no heuristic is used to pick a good split).
    pub fn insert(&mut self, polygons: &[Polygon]) {
        // build: function(polygons) {
        if polygons.is_empty() {
            return;
        }
        let SplitPolygonsResult {
            mut coplanar_front,
            mut coplanar_back,
            front,
            back,
        } = self.plane.split_polygons(polygons);

        self.polygons.append(&mut coplanar_front);
        self.polygons.append(&mut coplanar_back);
        if let Some(front_node) = &mut self.front {
            front_node.insert(&front);
        } else {
            self.front = Node::from_polygons(&front).map(Box::new);
        }

        if let Some(back_node) = &mut self.back {
            back_node.insert(&back);
        } else {
            self.back = Node::from_polygons(&back).map(Box::new);
        }
    }

    // Remove all polygons in this BSP tree that are inside the other BSP tree
    // `bsp`.
    pub fn clip_to(&mut self, other: &Node) {
        self.polygons = other.clip_polygons(&self.polygons);
        if let Some(front) = &mut self.front {
            front.clip_to(other);
        }
        if let Some(back) = &mut self.back {
            back.clip_to(other);
        }
    }

    // Convert solid space to empty space and empty space to solid space.
    pub fn invert(&mut self) {
        self.polygons.iter_mut().for_each(Polygon::flip);
        self.plane.flip();
        if let Some(front) = &mut self.front {
            front.invert();
        }
        if let Some(back) = &mut self.back {
            back.invert();
        }
        std::mem::swap(&mut self.front, &mut self.back);
    }

    pub fn all_polygons(&self) -> Vec<Polygon> {
        let mut polygons = self.polygons.clone();
        if let Some(front) = &self.front {
            polygons.append(&mut front.all_polygons());
        }
        if let Some(back) = &self.back {
            polygons.append(&mut back.all_polygons());
        }
        polygons
    }

    // Recursively remove all polygons in `polygons` that are inside this BSP
    // tree.
    fn clip_polygons(&self, polygons: &[Polygon]) -> Vec<Polygon> {
        let (front, back) = self.plane.split_polygons(polygons).into_merged();

        let mut front = if let Some(front_node) = &self.front {
            front_node.clip_polygons(&front)
        } else {
            front
        };

        let mut back = if let Some(back_node) = &self.back {
            back_node.clip_polygons(&back)
        } else {
            Vec::new()
        };

        front.append(&mut back);
        front
    }
}

pub fn union(a: &Csg, b: &Csg) -> Option<Csg> {
    // Return a new CSG solid representing space in either this solid or in the
    // solid `csg`. Neither this solid nor the solid `csg` are modified.
    //
    //     A.union(B)
    //
    //     +-------+            +-------+
    //     |       |            |       |
    //     |   A   |            |       |
    //     |    +--+----+   =   |       +----+
    //     +----+--+    |       +----+       |
    //          |   B   |            |       |
    //          |       |            |       |
    //          +-------+            +-------+

    if let (Some(mut a), Some(mut b)) = (
        Node::from_polygons(&a.polygons),
        Node::from_polygons(&b.polygons),
    ) {
        a.clip_to(&b);
        b.clip_to(&a);
        b.invert();
        b.clip_to(&a);
        b.invert();
        a.insert(&b.all_polygons());
        Some(Csg::from_polygons(a.all_polygons()))
    } else {
        None
    }
    //   union: function(csg) {
    //     var a = new CSG.Node(this.clone().polygons);
    //     var b = new CSG.Node(csg.clone().polygons);
    //     a.clip_to(b);
    //     b.clip_to(a);
    //     b.invert();
    //     b.clip_to(a);
    //     b.invert();
    //     a.build(b.all_polygons());
    //     return CSG.fromPolygons(a.all_polygons());
    //   },
}

// Return a new CSG solid representing space in this solid but not in the
// solid `csg`. Neither this solid nor the solid `csg` are modified.
//
//     A.subtract(B)
//
//     +-------+            +-------+
//     |       |            |       |
//     |   A   |            |       |
//     |    +--+----+   =   |    +--+
//     +----+--+    |       +----+
//          |   B   |
//          |       |
//          +-------+
//
pub fn subtract(a: &Csg, b: &Csg) -> Option<Csg> {
    if let (Some(mut a), Some(mut b)) = (
        Node::from_polygons(&a.polygons),
        Node::from_polygons(&b.polygons),
    ) {
        a.invert();
        a.clip_to(&b);
        b.clip_to(&a);
        b.invert();
        b.clip_to(&a);
        b.invert();
        a.insert(&b.all_polygons());
        a.invert();
        Some(Csg::from_polygons(a.all_polygons()))
    } else {
        None
    }
}

impl From<&Csg> for Mesh {
    fn from(csg: &Csg) -> Self {
        TriangleSlice(&csg.get_triangles_no_appearance()).into()
    }
}

struct TriangleSlice<'a>(&'a [([Vec3; 3], Vec3)]);

impl<'a> From<TriangleSlice<'a>> for Mesh {
    fn from(ts: TriangleSlice<'a>) -> Self {
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut uvs = Vec::new();
        let mut indices = Vec::new();

        let texgen = Texgen::default();
        for tri in ts.0 {
            let idx0 = positions.len() as u32;
            // most obnoxiously functional style just for the lulz...
            fn to_slice(v: Vec3) -> [f32; 3] {
                [v.x, v.y, v.z]
            }
            fn to_slice2(v: Vec2) -> [f32; 2] {
                [v.x, v.y]
            }
            positions.extend(tri.0.map(to_slice));
            let normal = tri.1.normalize();
            normals.extend(std::iter::repeat(to_slice(normal)).take(3));
            uvs.extend(
                tri.0
                    .map(|pos| texgen.project_tc_for_pos(pos, normal))
                    .map(to_slice2),
            );
            indices.extend(idx0..=(idx0 + 2));
        }

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.set_indices(Some(Indices::U32(indices)));
        mesh.generate_tangents().expect("generate tangents failed");
        mesh
    }
}

pub fn csg_to_split_meshes(csg: &Csg) -> Vec<(i32, Mesh)> {
    let triangles = csg.get_triangles();

    let mut id_to_triangles = HashMap::<i32, Vec<([Vec3; 3], Vec3)>>::new();

    // separate triangles per appearance id
    for (tri, normal, id) in triangles {
        match id_to_triangles.entry(id) {
            bevy::utils::hashbrown::hash_map::Entry::Occupied(mut e) => {
                e.get_mut().push((tri, normal));
            }
            bevy::utils::hashbrown::hash_map::Entry::Vacant(e) => {
                e.insert(vec![(tri, normal)]);
            }
        }
    }

    id_to_triangles
        .drain()
        .map(|(k, v)| (k, TriangleSlice(&v).into()))
        .collect()
}

#[test]
pub fn test_cube() {
    let cube1 = Cube::default();
    let csg1: Csg = cube1.into();

    println!("csg1: {:?}", csg1);

    let cube2 = Cube::new(Vec3::new(0.5, 0.0, 0.0), 1.0);
    let csg2: Csg = cube2.into();

    let csg3 = union(&csg1, &csg2).unwrap();

    println!("union: {:?}", csg3);

    println!("size: {} {}", csg1.polygons.len(), csg3.polygons.len());
}
