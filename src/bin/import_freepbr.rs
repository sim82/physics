use std::{
    f32::consts::E,
    io::{BufReader, Cursor, Read},
    path::{Path, PathBuf},
};

use bevy::{
    prelude::Vec3,
    render::texture::{ImageFormat, ImageType},
    utils::HashMap,
};
use clap::{builder::OsStr, Parser};
use image::{DynamicImage, ImageBuffer, Rgb};
use log::{info, warn};
use physics::material;
use zip::read::ZipArchive;

#[derive(clap::Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct CmdlineArgs {
    pbr_zip: PathBuf,

    #[clap(short, long)]
    asset_dir: PathBuf,

    #[clap(short, long)]
    name: Option<String>,

    #[clap(short, long)]
    existing: bool,
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum ImageKind {
    Albedo,
    Normal,
    Ao,
    Roughness,
    Metallic,
    Emissive,
}

impl ImageKind {
    pub fn guess_kind(name: &str) -> Option<ImageKind> {
        let lower = name.to_lowercase();
        if lower.contains("albedo") || lower.contains("basecolor") {
            Some(ImageKind::Albedo)
        } else if lower.contains("emissive") {
            Some(ImageKind::Emissive)
        } else if lower.contains("normal") || lower.contains("nmap") {
            println!("lower: {}", lower);

            if !lower.contains("ogl") {
                warn!("not 'ogl' in normal map. Check handedness.: {}", name);
            }
            Some(ImageKind::Normal)
        } else if lower.contains("rough") {
            Some(ImageKind::Roughness)
        } else if lower.contains("metal") {
            Some(ImageKind::Metallic)
        } else if lower.contains("ao") || lower.contains("ambient_occlusion") {
            Some(ImageKind::Ao)
        } else {
            None
        }
    }
}

fn main() {
    env_logger::init();
    let args = CmdlineArgs::parse();
    println!("{:?}", args.pbr_zip);

    // let Some(pbr_zip) = args.pbr_zip else {

    // }

    let name = match args.name {
        Some(name) => name,
        None => {
            let guessed_name = guess_name(&args.pbr_zip);
            let input = dialoguer::Input::new()
                .with_prompt("material name:")
                .with_initial_text(guessed_name)
                .interact_text();

            input.expect("failed to get material name")
        }
    };

    let image_dir = args.asset_dir.join("images").join(&name);

    if image_dir.exists() {
        println!("outdir exists: {:?}", image_dir);
        if !args.existing {
            std::process::exit(1)
        }
    } else {
        std::fs::create_dir_all(&image_dir)
            .unwrap_or_else(|_| panic!("create_dir failed: {:?}", image_dir));
    }
    if !image_dir.is_dir() {
        println!("outdir is not a directory: {:?}", image_dir);
        std::process::exit(1);
    }

    let material_name = args
        .asset_dir
        .join("materials")
        .join(format!("{}.ron", name));

    if material_name.exists() {
        println!("material name exists {:?}", material_name);
        if !args.existing {
            std::process::exit(1)
        }
    }

    let mut file_names = HashMap::new();
    // let image = Vec::new();

    let is_zip = args.pbr_zip.extension().map_or(false, |s| s == "zip");

    if is_zip {
        let zip_file = std::fs::File::open(args.pbr_zip).expect("failed to open zip file");
        let mut zip_archive = ZipArchive::new(zip_file).expect("failed to open zip archive");
        let names = zip_archive
            .file_names()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        file_names.extend(names.iter().filter_map(|s| {
            let name = s.to_string();
            let kind = ImageKind::guess_kind(&name)?;
            Some((
                kind,
                (name.clone(), read_image_from_zip(&mut zip_archive, &name)),
            ))
        }));
        // println!("{:?}", file_names);
    } else {
        todo!()
    }

    let (albedo, albedo_image) = file_names
        .remove(&ImageKind::Albedo)
        .expect("missing albedo image");

    let (normal, normal_image) = file_names
        .remove(&ImageKind::Normal)
        .expect("missing normal image");

    let (roughness, roughness_image) = file_names
        .remove(&ImageKind::Roughness)
        .expect("missing roughness image");

    // let mut albedo = None;
    // let mut albedo_image = None;
    // let mut ao = None;
    // let mut ao_image = None;
    // let mut metallic = None;
    // let mut metallic_image = None;
    // let mut roughness = None;
    // let mut roughness_image = None;
    // let mut normal = None;
    // let mut normal_image = None;
    // let mut emissive = None;
    // let mut emissive_image = None;

    // for (name, image) in file_names {
    //     let lower = name.to_lowercase();
    //     if lower.contains("albedo") || lower.contains("basecolor") {
    //         albedo = Some(name);
    //         albedo_image = Some(image);
    //     } else if lower.contains("emissive") {
    //         emissive = Some(name);
    //         emissive_image = Some(image);
    //     } else if lower.contains("normal") || lower.contains("nmap") {
    //         println!("lower: {}", lower);

    //         if !lower.contains("ogl") {
    //             warn!("not 'ogl' in normal map. Check handedness.: {}", name);
    //         }
    //         normal = Some(name);
    //         normal_image = Some(image);
    //     } else if lower.contains("rough") {
    //         roughness = Some(name);
    //         roughness_image = Some(image);
    //     } else if lower.contains("metal") {
    //         metallic = Some(name);
    //         metallic_image = Some(image);
    //     } else if lower.contains("ao") || lower.contains("ambient_occlusion") {
    //         ao = Some(name);
    //         ao_image = Some(image);
    //     }
    // }

    // check_file("albedo", &albedo, true);
    // check_file("normal", &normal, true);
    // check_file("roughness", &roughness, true);
    // check_file("ao", &ao, false);
    // check_file("metallic", &metallic, false);
    // check_file("emissive", &emissive, false);

    // let albedo_image = albedo_image.expect("missing albedo image");
    println!("albedo image: {:?}", albedo_image.color());
    // let normal_image = normal_image.expect("missing normal image");

    println!("normal image: {:?}", normal_image.color());
    // let roughness_image = roughness_image.expect("missing roughness image");
    println!("roughness image: {:?}", roughness_image.color());

    let rm_image = if let Some((metallic, metallic_image)) = file_names.remove(&ImageKind::Metallic)
    {
        println!("metallic image: {:?}", metallic_image.color());
        let m = metallic_image.into_luma8(); //.expect("as_rgb8 failed");
        let r = roughness_image.into_luma8(); // .expect("as_rgb8 failed");
        ImageBuffer::from_fn(albedo_image.width(), albedo_image.height(), |x, y| {
            Rgb::<u8>([0, r.get_pixel(x, y).0[0], m.get_pixel(x, y).0[0]])
        })
    } else {
        roughness_image.into_rgb8()
    };
    // let albedo_image = albedo_image.into_rgb8();
    // let normal_image = normal_image.into_rgb8();

    let albedo_output = format!("{}_albedo.png", name);
    let normal_output = format!("{}_normal.norm", name);
    let ao_output = format!("{}_ao.norm", name);
    let mr_output = format!("{}_mr.norm", name);
    let emissive_output = format!("{}_emissive.norm", name);

    // let albedo_style = console::Style::new().green();
    // println!("{}", albedo_style.apply_to("albedo"));

    albedo_image
        .save_with_format(image_dir.join(&albedo_output), image::ImageFormat::Png)
        .expect("failed tp write albedo image");
    normal_image
        .save_with_format(image_dir.join(&normal_output), image::ImageFormat::Png)
        .expect("failed tp write normal image");
    rm_image
        .save_with_format(image_dir.join(&mr_output), image::ImageFormat::Png)
        .expect("failed tp rm albedo image");

    if let Some((ao, ao_image)) = file_names.remove(&ImageKind::Ao) {
        println!("ao image: {:?}", ao_image.color());

        let ao_image = ao_image.into_luma8();

        ao_image
            .save_with_format(image_dir.join(&ao_output), image::ImageFormat::Png)
            .expect("failed tp write ao image");
    }

    if let Some((emissive, emissive_image)) = file_names.get(&ImageKind::Emissive) {
        emissive_image
            .save_with_format(image_dir.join(&emissive_output), image::ImageFormat::Png)
            .expect("failed to write emissive image");
    }

    let material = material::Material {
        base: Some(format!("images/{}/{}", name, albedo_output)),
        occlusion: if file_names.contains_key(&ImageKind::Ao) {
            Some(format!("images/{}/{}", name, ao_output))
        } else {
            None
        },
        normal_map: Some(format!("images/{}/{}", name, normal_output)),
        metallic_roughness_texture: Some(format!("images/{}/{}", name, mr_output)),
        metallic: Some(1.0),
        roughness: Some(1.0),
        base_color: None,
        emissive: if file_names.contains_key(&ImageKind::Emissive) {
            Some(format!("images/{}/{}", name, emissive_output))
        } else {
            None
        },
        emissive_color: if file_names.contains_key(&ImageKind::Emissive) {
            Some(Vec3::ONE)
        } else {
            None
        },
        reflectance: None,
    };
    if let Ok(f) = std::fs::File::create(material_name) {
        let choices = ["appearance/test/whiteconcret3", "appearance/test/con52_1"];
        let idx = dialoguer::Select::new()
            .items(&choices)
            .interact()
            .expect("select failed");

        let mut map = HashMap::new();
        map.insert(choices[idx], material);
        ron::ser::to_writer_pretty(f, &map, Default::default()).expect("failed");
    }
}

fn check_file(t: &str, name: &Option<String>, necessary: bool) {
    if necessary {
        match name.clone() {
            Some(name) => {
                let style = console::Style::new().green();
                println!("{}", style.apply_to(format!("{}: {}", t, name)))
            }
            None => {
                let style = console::Style::new().red();
                println!("{}", style.apply_to(format!("{}: missing", t)));
                std::process::exit(1);
            }
        }
    } else {
        match name.clone() {
            Some(name) => {
                let style = console::Style::new().green();
                println!("{}", style.apply_to(format!("{}: {}", t, name)))
            }
            None => {
                let style = console::Style::new().yellow();
                println!(
                    "{}",
                    style.apply_to(format!("{}: not available (optional)", t))
                );
            }
        }
    }
}

fn guess_name(pbr_zip: &PathBuf) -> String {
    let prefix = pbr_zip
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    if prefix.ends_with("-bl") {
        prefix.trim_end_matches("-bl").to_string()
    } else {
        prefix
    }
}

fn read_image_from_zip(
    zip_archive: &mut ZipArchive<std::fs::File>,
    name: &String,
) -> image::DynamicImage {
    let albedo_zip = zip_archive
        .by_name(name)
        .unwrap_or_else(|_| panic!("failed to get image zip entry: {}", name));
    read_image(albedo_zip)
}

fn read_image(mut reader: impl Read) -> DynamicImage {
    let mut data = Vec::new();
    reader.read_to_end(&mut data).expect("read failed");
    image::load_from_memory(&data[..]).expect("image load failed")
}
