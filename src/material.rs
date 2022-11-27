use std::path::Path;

use bevy::{prelude::*, utils::HashMap};
use serde::{Deserialize, Serialize};

use crate::appearance::{self};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Material {
    pub base: Option<String>,
    pub base_color: Option<Vec3>,
    pub emissive: Option<String>,
    pub emissive_color: Option<Vec3>,
    pub roughness: Option<f32>,
    pub metallic: Option<f32>,
    pub metallic_roughness_texture: Option<String>,
    pub reflectance: Option<f32>,
    pub normal_map: Option<String>,
    pub occlusion: Option<String>,
}

pub fn load_all_material_files<P: AsRef<Path>>(dir: P) -> HashMap<String, Material> {
    let mut res = HashMap::new();

    for e in std::fs::read_dir(dir).unwrap() {
        let Ok(ent) = e else {continue};
        if !ent.file_type().unwrap().is_file()
            || !ent.file_name().to_string_lossy().ends_with(".ron")
        {
            continue;
        }

        let f = std::fs::File::open(ent.path()).unwrap();

        let mut apps = match ron::de::from_reader::<_, HashMap<String, Material>>(f) {
            Ok(apps) => apps,
            Err(e) => {
                warn!("failed to load {:?}: {:?}", ent.path(), e);
                continue;
            }
        };
        res.extend(apps.drain());
    }

    res
}

pub fn load_image(image: Option<String>, asset_server: &mut AssetServer) -> Option<Handle<Image>> {
    Some(asset_server.load(image?))
}

pub fn load_materials(
    base_dir: impl AsRef<Path>,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &mut AssetServer,
) -> HashMap<String, Handle<StandardMaterial>> {
    let apps = load_all_material_files(base_dir.as_ref().join("materials"));
    // let image_dir = base_dir.as_ref().join("image");
    let mut images: HashMap<String, Handle<Image>> = HashMap::new();
    let mut res = HashMap::new();
    for (name, material) in apps {
        let material = materials.add(StandardMaterial {
            base_color_texture: load_image(material.base, asset_server),
            perceptual_roughness: material.roughness.unwrap_or(0.089),
            metallic: material.roughness.unwrap_or(0.001),

            normal_map_texture: load_image(material.normal_map, asset_server),
            metallic_roughness_texture: load_image(
                material.metallic_roughness_texture,
                asset_server,
            ),
            occlusion_texture: load_image(material.occlusion, asset_server),
            emissive_texture: load_image(material.emissive, asset_server),
            emissive: material
                .emissive_color
                .map(|c| c.extend(1.0).into())
                .unwrap_or(Color::BLACK),
            ..default()
        });
        res.insert(name, material);
    }
    res
}

#[test]
fn test() {
    let mat = Material {
        base: Some("base".into()),
        base_color: Some(Vec3::ZERO),
        emissive: Some("emissive".into()),
        emissive_color: Some(Vec3::ZERO),
        roughness: Some(1.0),
        metallic: Some(1.0),
        metallic_roughness_texture: Some("mrt".into()),
        reflectance: Some(1.0),
        normal_map: Some("norm".into()),
        occlusion: Some("occlusion".into()),
    };
    let m: HashMap<_, _> = [
        ("test1".to_string(), mat.clone()),
        ("test2".to_string(), mat.clone()),
    ]
    .iter()
    .cloned()
    .collect();

    let t = ron::ser::to_string_pretty(&m, default()).unwrap();
    // let t = serde_yaml::to_string(&m).unwrap();
    println!("{}", t);
}

#[test]
fn test2() {
    let mat: Material = ron::de::from_str("(base: \"blub\")").unwrap();
    println!("{:?}", mat);
}
