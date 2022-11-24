// "appearance/metal/metal60_8": {
//     "theClass": "appearance",
//     "theName": "default",
//     "primaryImage": "image/metal/metal60_8",
//     "meshClassName": "mesh/test_tiles/quad",
//     "shaderClass": "TextureTest",
//     "size": "2.000 2.000 0.0",
//     "layerConfig": [
//       {
//         "name": "layer0",
//         "projector": "surfaceProjector0"
//       },
//       {
//         "name": "layer1",
//         "projector": "smtile"
//       }
//     ],
//     "shaderConfig": {
//       "image": "image/metal/metal60_8",
//       "bumpmap": "image/metal/metal60_8_bump_h8",
//       "pass": "0"
//     }
//   },

use std::path::Path;

use bevy::{prelude::*, utils::HashMap};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Appearance {
    pub theClass: Option<String>,
    pub theName: Option<String>,
    pub primaryImage: String,
    pub size: String,
    pub shaderConfig: ShaderConfig,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ShaderConfig {
    pub image: Option<String>,
    pub bumpmap: Option<String>,
    pub specColor: Option<String>,
    pub specExp: Option<String>,
}

pub fn load_all_appearance_files<P: AsRef<Path>>(dir: P) -> HashMap<String, Appearance> {
    let mut res = HashMap::new();

    for e in std::fs::read_dir("/home/sim/3dyne/arch00.dir/appearance/").unwrap() {
        let Ok(ent) = e else {continue};
        if !ent.file_type().unwrap().is_file()
            || !ent.file_name().to_string_lossy().ends_with(".json")
        {
            continue;
        }

        let f = std::fs::File::open(ent.path()).unwrap();

        let mut apps = match serde_json::from_reader::<_, HashMap<String, Appearance>>(f) {
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

pub fn load_materials(
    base_dir: impl AsRef<Path>,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &mut AssetServer,
) -> HashMap<String, Handle<StandardMaterial>> {
    let apps = load_all_appearance_files(base_dir.as_ref().join("appearance"));
    // let image_dir = base_dir.as_ref().join("image");
    let mut images: HashMap<String, Handle<Image>> = HashMap::new();
    let mut res = HashMap::new();

    for (name, appearance) in apps {
        let Some(image) = &appearance.shaderConfig.image else {continue};

        let image_res = load_image(image, &base_dir, &mut images, asset_server);
        // let image_res = None;
        let normal_map = if appearance.shaderConfig.bumpmap.is_some() {
            load_image(
                // "image/test/TestNormalMap",
                "image/wall/con52_1_normal",
                &base_dir,
                &mut images,
                asset_server,
            )
        } else {
            None
        };

        let material = materials.add(StandardMaterial {
            base_color_texture: image_res,
            perceptual_roughness: 0.9,
            normal_map_texture: normal_map,
            ..default()
        });
        res.insert(name, material);
    }
    res
}

pub fn load_image(
    image: &str,
    base_dir: &impl AsRef<Path>,
    images: &mut HashMap<String, Handle<Image>>,

    asset_server: &mut AssetServer,
) -> Option<Handle<Image>> {
    let image_res = match images.entry(image.to_string()) {
        bevy::utils::hashbrown::hash_map::Entry::Occupied(e) => Some(e.get().clone()),
        bevy::utils::hashbrown::hash_map::Entry::Vacant(e) => {
            let image_stump = base_dir.as_ref().join(image);
            info!("load: {:?}", image_stump.with_extension("png"));

            if image_stump.with_extension("norm").exists() {
                Some(
                    e.insert(asset_server.load(image_stump.with_extension("norm")))
                        .clone(),
                )
            } else if image_stump.with_extension("png").exists() {
                Some(
                    e.insert(asset_server.load(image_stump.with_extension("png")))
                        .clone(),
                )
            } else if image_stump.with_extension("jpg").exists() {
                Some(
                    e.insert(asset_server.load(image_stump.with_extension("jpg")))
                        .clone(),
                )
            } else {
                None
            }
        }
    };
    image_res
}
