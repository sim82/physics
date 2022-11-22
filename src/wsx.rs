use std::{io::BufReader, path::Path};

use bevy::prelude::Vec3;
use serde::{Deserialize, Serialize};

use crate::csg;
use bevy::prelude::*;

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
pub struct WiredExportScene {
    pub format: String,
    pub SceneNodes: SceneNodes,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
pub struct SceneNodes {
    pub SceneNode: Vec<SceneNode>,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
pub struct SceneNode {
    pub id: i32,
    pub def: String,
    pub Properties: Properties,
    pub Components: Components,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
pub struct Properties {
    pub origin: String,
    pub csgLevel: Option<i32>,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
pub struct Components {
    pub Brush: Option<Vec<Brush>>,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
pub struct Brush {
    pub numSurfaces: i32,
    pub Surface: Vec<Surface>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Surface {
    pub id: String,
    pub origin: String,
    pub normal: String,
    pub appearance: String,
    pub translate: String,
    pub scale: String,
    pub shift: String,
    pub rotate: String,
    pub drotate: String,
}

fn parse_vec3(s: &str) -> Option<Vec3> {
    let mut f = s
        .split_ascii_whitespace()
        .filter_map(|s| s.parse::<f32>().ok());
    Some(Vec3::new(f.next()?, f.next()?, f.next()?))
}

impl Surface {
    pub fn to_csg_plane_with_offset(&self, offset: &Vec3) -> csg::Plane {
        let origin = parse_vec3(&self.origin).unwrap() + *offset;
        let normal = parse_vec3(&self.normal).unwrap();

        // TODO: read some linalg stuff and figure out if this is the right way to do this:
        // - project origin onto normal. this should be roughly the point in the plane closest to (0,0,0)
        // - note: projecting onto negative basis vectors does not automatically flip the sign of the projection!
        // - 'flip sign' if projected vector points in the opposite direction of the normal (-> dot)
        // - use length of that abomination as w
        let proj = origin.project_onto(normal);
        let w = proj.length() * proj.normalize_or_zero().dot(normal);

        debug!("{:?} {:?} -> {:?} {}", origin, normal, proj, w);
        csg::Plane {
            normal: parse_vec3(&self.normal).unwrap(),
            w,
        }
    }
}

impl Brush {
    pub fn to_csg_brush_with_offset(&self, offset: &Vec3) -> csg::Brush {
        info!("brush: {:?}", offset);
        csg::Brush {
            planes: self
                .Surface
                .iter()
                .map(|s| s.to_csg_plane_with_offset(offset))
                .collect(),
        }
    }
}

pub fn load_brushes<F: AsRef<Path>>(filename: F) -> Vec<csg::Brush> {
    let file = std::fs::File::open(filename).unwrap();
    println!("res");
    let wsx: WiredExportScene = quick_xml::de::from_reader(BufReader::new(file)).unwrap();
    // println!("wsx: {:?}", wsx);

    let mut res = Vec::new();
    for node in &wsx.SceneNodes.SceneNode {
        // ignore csg level 2 or higher
        if !matches!(node.Properties.csgLevel, None | Some(1)) {
            continue;
        }

        let origin = parse_vec3(&node.Properties.origin).unwrap();

        let Some(brushes) = &node.Components.Brush else {continue};
        for brush in brushes {
            let csg_brush: csg::Brush = brush.to_csg_brush_with_offset(&origin);

            println!("{:?}", csg_brush);
            res.push(csg_brush);
        }
    }
    res
}
