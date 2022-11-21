use std::{io::BufReader, path::Path};

use bevy::prelude::{Deref, Vec3};
use serde::{Deserialize, Serialize};

use crate::csg;

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

#[derive(Serialize, Deserialize, Debug)]
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

impl From<&Surface> for crate::csg::Plane {
    fn from(surface: &Surface) -> Self {
        let origin = parse_vec3(&surface.origin).unwrap();
        let normal = parse_vec3(&surface.normal).unwrap();

        let w = (-origin).project_onto(normal).length();

        csg::Plane {
            normal: parse_vec3(&surface.normal).unwrap(),
            w: w,
        }
    }
}

impl From<&Brush> for crate::csg::Brush {
    fn from(brush: &Brush) -> Self {
        csg::Brush {
            planes: brush.Surface.iter().map(|s| s.into()).collect(),
        }
    }
}

pub fn load_brushes<F: AsRef<Path>>(filename: F) -> Vec<csg::Brush> {
    let file = std::fs::File::open(filename).unwrap();
    println!("res");
    let wsx: WiredExportScene = quick_xml::de::from_reader(BufReader::new(file)).unwrap();
    println!("wsx: {:?}", wsx);

    let mut res = Vec::new();
    for node in &wsx.SceneNodes.SceneNode {
        if node.Properties.csgLevel.is_some() {
            continue;
        }
        let Some(brushes) = &node.Components.Brush else {continue};
        for brush in brushes {
            let csg_brush: csg::Brush = brush.into();

            println!("{:?}", csg_brush);
            res.push(csg_brush);
        }
    }
    res
}
