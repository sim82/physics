use bevy::{
    prelude::*,
    render::{mesh::Indices, render_resource::PrimitiveTopology},
};

// clean slate, bevy flavoured, port of csg.js

#[derive(Clone, Copy, Default, Debug)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
}

impl Vertex {
    pub fn new(position: Vec3, normal: Vec3) -> Self {
        Vertex { position, normal }
    }
    pub fn flip(&mut self) {
        self.normal = -self.normal;
    }
    pub fn interpolated(&self, other: &Vertex, f: f32) -> Self {
        Self {
            position: self.position.lerp(other.position, f),
            normal: self.normal.lerp(other.normal, f),
        }
    }
}

pub const PLANE_EPSILON: f32 = 1e-5;

#[derive(Clone, Debug, Default, Copy)]
pub struct Plane {
    pub normal: Vec3,
    pub w: f32,
}

// #[derive(Clone, Copy, Debug, Eq, PartialEq)]
// pub enum Location {
//     Coplanar,
//     Front,
//     Back,
//     Spanning,
// }

bitflags::bitflags! {
    struct Location : u32 {
        const NONE = 0;
        const COPLANAR = Self::NONE.bits();
        const FRONT = 1;
        const BACK = 2;
        const SPANNING = Self::FRONT.bits() | Self::BACK.bits;
    }
}

impl Plane {
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
    pub fn split_polygon(
        &self,
        polygon: &Polygon,
        coplanar_front: &mut Vec<Polygon>,
        coplanar_back: &mut Vec<Polygon>,
        front: &mut Vec<Polygon>,
        back: &mut Vec<Polygon>,
    ) {
        let mut polygon_type = Location::NONE;
        let mut types = Vec::new();

        for v in &polygon.vertices {
            let t = self.normal.dot(v.position) - self.w;
            let location = if t < -PLANE_EPSILON {
                Location::BACK
            } else if t > PLANE_EPSILON {
                Location::FRONT
            } else {
                Location::COPLANAR
            };

            polygon_type |= location;
            types.push(location);
        }
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
                    front.push(Polygon::from_vertices(f))
                }

                if b.len() >= 3 {
                    back.push(Polygon::from_vertices(b))
                }
                // for (var i = 0; i < polygon.vertices.length; i++) {
                //   var j = (i + 1) % polygon.vertices.length;
                //   var ti = types[i], tj = types[j];
                //   var vi = polygon.vertices[i], vj = polygon.vertices[j];
                //   if (ti != BACK) f.push(vi);
                //   if (ti != FRONT) b.push(ti != BACK ? vi.clone() : vi);
                //   if ((ti | tj) == SPANNING) {
                //     var t = (this.w - this.normal.dot(vi.pos)) / this.normal.dot(vj.pos.minus(vi.pos));
                //     var v = vi.interpolate(vj, t);
                //     f.push(v);
                //     b.push(v.clone());
                //   }
                // }
                // if (f.length >= 3) front.push(new CSG.Polygon(f, polygon.shared));
                // if (b.length >= 3) back.push(new CSG.Polygon(b, polygon.shared));
                // break
            }
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Polygon {
    pub vertices: Vec<Vertex>,
    pub plane: Plane,
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

    pub fn from_vertices(vertices: Vec<Vertex>) -> Polygon {
        Polygon {
            plane: Polygon::plane_from_vertices(&vertices[0..3]),
            vertices,
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

#[derive(Clone, Debug, Default)]
pub struct Csg {
    pub polygons: Vec<Polygon>,
}

impl Csg {
    pub fn from_polygons(polygons: Vec<Polygon>) -> Self {
        Csg { polygons }
    }

    // pub fn get_triangles(&self) -> Vec<Triangle> {
    //     let mut result = Vec::new();

    //     for poly in &self.polygons {
    //         for i in 1..(poly.vertices.len() - 1) {
    //             let v0 = poly.vertices[0].position;
    //             let v1 = poly.vertices[i].position;
    //             let v2 = poly.vertices[i + 1].position;

    //             result.push(Triangle {
    //                 positions: [v0, v1, v2],
    //                 normal: poly.plane.0,
    //             });
    //         }
    //     }

    //     result
    // }

    pub fn get_triangles(&self) -> Vec<([Vec3; 3], Vec3)> {
        let mut res = Vec::new();

        for p in &self.polygons {
            if p.vertices.len() < 3 {
                continue;
            }
            // premature and completely unnecessary optimization
            res.reserve(p.vertices.len() - 2);

            // crate triangle 'fans': all triangles share 1st point
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

    pub fn invert(&mut self) {
        for p in &mut self.polygons {
            p.flip();
        }
    }
}

#[derive(Clone, Debug, Default)]
struct Node {
    pub plane: Plane,
    pub front: Option<Box<Node>>,
    pub back: Option<Box<Node>>,
    pub polygons: Vec<Polygon>,
}

impl Node {
    pub fn from_polygons(polygons: &Vec<Polygon>) -> Option<Node> {
        // build: function(polygons) {
        if polygons.is_empty() {
            return None;
        }
        // if (!polygons.length) return;
        // if (!this.plane) this.plane = polygons[0].plane.clone();
        let plane = polygons[0].plane;
        // var front = [], back = [];
        let mut front = Vec::new();
        let mut back = Vec::new();
        let mut coplanar_front = Vec::new();
        let mut coplanar_back = Vec::new();

        for p in polygons {
            plane.split_polygon(
                p,
                &mut coplanar_front,
                &mut coplanar_back,
                &mut front,
                &mut back,
            );
        }

        let mut polygons = coplanar_front;
        polygons.append(&mut coplanar_back);
        // if !front.is_empty() {
        //     if !
        // }

        let front = Node::from_polygons(&front).map(Box::new);
        let back = Node::from_polygons(&back).map(Box::new);
        // let front = Box::new(Node::from_polygons(front));
        // let back = Box::new(Node::from_polygons(back));

        Some(Node {
            plane,
            front,
            back,
            polygons,
        })

        // if (front.length) {
        //   if (!this.front) this.front = new CSG.Node();
        //   this.front.build(front);
        // }
        // if (back.length) {
        //   if (!this.back) this.back = new CSG.Node();
        //   this.back.build(back);
        // }
    }

    pub fn insert(&mut self, polygons: &Vec<Polygon>) {
        // build: function(polygons) {
        if polygons.is_empty() {
            return;
        }
        let mut front = Vec::new();
        let mut back = Vec::new();

        let mut coplanar_front = Vec::new();
        let mut coplanar_back = Vec::new();

        for p in polygons {
            self.plane.split_polygon(
                p,
                &mut coplanar_front,
                &mut coplanar_back,
                &mut front,
                &mut back,
            );
        }

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

        // if !front.is_empty() {
        //     if !
        // }

        // let front = Node::from_polygons(front).map(Box::new);
        // let back = Node::from_polygons(back).map(Box::new);
        // let front = Box::new(Node::from_polygons(front));
        // let back = Box::new(Node::from_polygons(back));

        // if (front.length) {
        //   if (!this.front) this.front = new CSG.Node();
        //   this.front.build(front);
        // }
        // if (back.length) {
        //   if (!this.back) this.back = new CSG.Node();
        //   this.back.build(back);
        // }
    }

    // Remove all polygons in this BSP tree that are inside the other BSP tree
    // `bsp`.
    pub fn clip_to(&mut self, other: &Node) {
        // this.polygons = bsp.clipPolygons(this.polygons);
        // if (this.front) this.front.clipTo(bsp);
        // if (this.back) this.back.clipTo(bsp);
        self.polygons = other.clip_polygons(&self.polygons);
        if let Some(front) = &mut self.front {
            front.clip_to(other);
        }
        if let Some(back) = &mut self.back {
            back.clip_to(other);
        }
    }

    fn invert(&mut self) {
        self.polygons.iter_mut().for_each(Polygon::flip);
        self.plane.flip();
        if let Some(front) = &mut self.front {
            front.invert();
        }
        if let Some(back) = &mut self.back {
            back.invert();
        }
        std::mem::swap(&mut self.front, &mut self.back);
        //  // Convert solid space to empty space and empty space to solid space.
        //  invert: function() {
        //     for (var i = 0; i < this.polygons.length; i++) {
        //       this.polygons[i].flip();
        //     }
        //     this.plane.flip();
        //     if (this.front) this.front.invert();
        //     if (this.back) this.back.invert();
        //     var temp = this.front;
        //     this.front = this.back;
        //     this.back = temp;
        //   },
    }

    pub fn all_polygons(&self) -> Vec<Polygon> {
        // Return a list of all polygons in this BSP tree.
        //   allPolygons: function() {
        let mut polygons = self.polygons.clone();
        if let Some(front) = &self.front {
            polygons.append(&mut front.all_polygons());
        }
        if let Some(back) = &self.back {
            polygons.append(&mut back.all_polygons());
        }
        polygons
        // var polygons = this.polygons.slice();
        // if (this.front) polygons = polygons.concat(this.front.allPolygons());
        // if (this.back) polygons = polygons.concat(this.back.allPolygons());
        // return polygons;
        //   },
    }

    fn clip_polygons(&self, polygons: &Vec<Polygon>) -> Vec<Polygon> {
        let mut front = Vec::new();
        let mut back = Vec::new();
        let mut coplanar_front = Vec::new();
        let mut coplanar_back = Vec::new();

        for polygon in polygons {
            self.plane.split_polygon(
                polygon,
                &mut coplanar_front,
                &mut coplanar_back,
                &mut front,
                &mut back,
            )
        }

        front.append(&mut coplanar_front);
        back.append(&mut coplanar_back);

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
        //   // Recursively remove all polygons in `polygons` that are inside this BSP
        // // tree.
        // clipPolygons: function(polygons) {
        // if (!this.plane) return polygons.slice();
        // var front = [], back = [];
        // for (var i = 0; i < polygons.length; i++) {
        //   this.plane.splitPolygon(polygons[i], front, back, front, back);
        // }
        // if (this.front) front = this.front.clipPolygons(front);
        // if (this.back) back = this.back.clipPolygons(back);
        // else back = [];
        // return front.concat(back);
        //   },
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
    //     a.clipTo(b);
    //     b.clipTo(a);
    //     b.invert();
    //     b.clipTo(a);
    //     b.invert();
    //     a.build(b.allPolygons());
    //     return CSG.fromPolygons(a.allPolygons());
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
                Polygon::from_vertices(vtx)
            })
            .collect();

        Csg::from_polygons(polygons)

        // CSG.cube = function(options) {
        //     options = options || {};
        //     var c = new CSG.Vector(options.center || [0, 0, 0]);
        //     var r = !options.radius ? [1, 1, 1] : options.radius.length ?
        //              options.radius : [options.radius, options.radius, options.radius];
        //     return CSG.fromPolygons([
        //       [[0, 4, 6, 2], [-1, 0, 0]],
        //       [[1, 3, 7, 5], [+1, 0, 0]],
        //       [[0, 1, 5, 4], [0, -1, 0]],
        //       [[2, 6, 7, 3], [0, +1, 0]],
        //       [[0, 2, 3, 1], [0, 0, -1]],
        //       [[4, 5, 7, 6], [0, 0, +1]]
        //     ].map(function(info) {
        //       return new CSG.Polygon(info[0].map(function(i) {
        //         var pos = new CSG.Vector(
        //           c.x + r[0] * (2 * !!(i & 1) - 1),
        //           c.y + r[1] * (2 * !!(i & 2) - 1),
        //           c.z + r[2] * (2 * !!(i & 4) - 1)
        //         );
        //         return new CSG.Vertex(pos, new CSG.Vector(info[1]));
        //       }));
        //     }));
        //   };
    }
}

impl From<&Csg> for Mesh {
    fn from(csg: &Csg) -> Self {
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut uvs = Vec::new();
        let mut indices = Vec::new();

        let triangles = csg.get_triangles();
        for tri in &triangles {
            let idx0 = positions.len() as u32;
            // most obnoxiously functional style just for the lulz...
            fn to_slice(v: Vec3) -> [f32; 3] {
                [v.x, v.y, v.z]
            }
            positions.extend(tri.0.map(to_slice));
            normals.extend(std::iter::repeat(to_slice(tri.1)).take(3));
            uvs.extend(std::iter::repeat([0.0, 0.0]).take(3));
            indices.extend(idx0..=(idx0 + 2));
        }

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.set_indices(Some(Indices::U32(indices)));
        mesh
    }
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
