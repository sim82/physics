use std::{io::BufReader, path::Path};

use bevy::{prelude::Vec3, utils::HashMap};
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
    pub def: Option<String>,
    pub Properties: Properties,
    pub Components: Components,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
pub struct Properties {
    pub origin: String,
    pub csgLevel: Option<i32>,
    pub attenuationRadius: Option<f32>,
    pub thingdef: Option<String>,
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
    pub fn to_csg_brush_with_offset(&self, offset: &Vec3) -> (csg::Brush, Vec<&str>) {
        info!("brush: {:?}", offset);

        let appearances = self.Surface.iter().map(|s| s.appearance.as_str()).collect();
        (
            csg::Brush {
                planes: self
                    .Surface
                    .iter()
                    .map(|s| s.to_csg_plane_with_offset(offset))
                    .collect(),
                appearances: std::iter::repeat(0).take(self.Surface.len()).collect(),
            },
            appearances,
        )
    }
}

pub fn load_brushes<F: AsRef<Path>>(filename: F) -> (Vec<csg::Brush>, HashMap<i32, String>) {
    let file = std::fs::File::open(filename).unwrap();
    println!("res");
    let wsx: WiredExportScene = quick_xml::de::from_reader(BufReader::new(file)).unwrap();
    // println!("wsx: {:?}", wsx);

    let mut res = Vec::new();
    let mut appearances = HashMap::new();
    let mut next_appearance = 0;
    for node in &wsx.SceneNodes.SceneNode {
        // ignore csg level 2 or higher
        if node
            .def
            .clone()
            .or_else(|| node.Properties.thingdef.clone())
            .unwrap_or_default()
            != "CsgBrush"
            || !matches!(node.Properties.csgLevel, None | Some(1))
        {
            continue;
        }

        let origin = parse_vec3(&node.Properties.origin).unwrap();

        let Some(brushes) = &node.Components.Brush else {continue};
        for brush in brushes {
            let (mut csg_brush, plane_appearances) = brush.to_csg_brush_with_offset(&origin);

            // update appearance id in planes, establishing an name -> id map along the way
            for (appearance, name) in csg_brush.appearances.iter_mut().zip(plane_appearances) {
                *appearance = match appearances.entry(name) {
                    bevy::utils::hashbrown::hash_map::Entry::Occupied(e) => *e.get(),
                    bevy::utils::hashbrown::hash_map::Entry::Vacant(e) => {
                        let tmp = next_appearance;
                        next_appearance += 1;
                        *e.insert(tmp)
                    }
                };
            }

            println!("{:?}", csg_brush);
            res.push(csg_brush);
        }
    }
    // return the brushes along with a id -> name map for the appearances
    (
        res,
        appearances
            .drain()
            .map(|(k, v)| (v, k.to_string()))
            .collect(),
    )
}

pub fn load_pointlights<F: AsRef<Path>>(filename: F) -> Vec<(Vec3, f32)> {
    let file = std::fs::File::open(filename).unwrap();
    println!("res");
    let wsx: WiredExportScene = quick_xml::de::from_reader(BufReader::new(file)).unwrap();
    // println!("wsx: {:?}", wsx);

    let mut res = Vec::new();
    for node in &wsx.SceneNodes.SceneNode {
        // ignore csg level 2 or higher
        if node
            .def
            .clone()
            .or_else(|| node.Properties.thingdef.clone())
            .unwrap_or_default()
            != "PointLight"
        {
            continue;
        }

        let origin = parse_vec3(&node.Properties.origin).unwrap();
        let attenuation_radius = node.Properties.attenuationRadius.unwrap_or(10.0); // TODO: check what's the default

        res.push((origin, attenuation_radius));
    }
    res
}
