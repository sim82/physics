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
use bevy_inspector_egui::egui::output;
use clap::{builder::OsStr, Parser};
use image::{DynamicImage, ImageBuffer, Rgb};
use log::{info, warn};
use physics::material;
use zip::read::ZipArchive;

#[derive(clap::Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct CmdlineArgs {
    input_pbr: PathBuf,

    #[clap(short, long)]
    asset_dir: PathBuf,

    #[clap(short, long)]
    name: Option<String>,

    #[clap(short, long)]
    existing: bool,

    #[clap(short, long)]
    batch: bool,
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

        if !lower.ends_with(".png") || lower.contains("preview") {
            return None;
        }

        if lower.contains("albedo")
            || lower.contains("basecolor")
            || lower.contains("base_color")
            || lower.contains("default_color")
            || lower.ends_with("-alb.png")
        {
            Some(ImageKind::Albedo)
        } else if lower.contains("emissive") {
            Some(ImageKind::Emissive)
        } else if lower.contains("normal")
            || lower.contains("nmap")
            || lower.contains("norrmal")
            || lower.contains("norma-ogl")
        {
            // println!("lower: {}", lower);

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
    println!("{:?}", args.input_pbr);

    // let Some(pbr_zip) = args.pbr_zip else {

    // }

    if !args.batch {
        import_single_material(args.input_pbr, &args.name, args.asset_dir, args.existing);
    } else {
        // the freepbr bulk package is organized in filters like metals-bl/vertical-lined-metal-bl/*.png
        // so we can re-use the same filter for traversing both directory levels
        let dir_filter = |ent: Result<std::fs::DirEntry, _>| {
            let ent = ent.ok()?;

            if !ent.metadata().ok()?.is_dir() {
                return None;
            }
            // let file_name = ent.file_name();
            // let file_name = file_name.to_str()?;
            // if !file_name.ends_with("-bl") {
            //     return None;
            // }
            let dir_name_os = ent.file_name();
            let dir_name = dir_name_os.to_str()?;
            let stripped_dir_name = dir_name.strip_suffix("-bl")?;

            Some((stripped_dir_name.to_string(), ent))
        };

        for (category_name, ent) in std::fs::read_dir(args.input_pbr)
            .expect("read_dir failed")
            .filter_map(dir_filter)
        {
            let materials_dir = args.asset_dir.join("materials");
            let material_path = materials_dir.join(format!("{}.ron", category_name));
            let mut materials = if let Ok(f) = std::fs::File::open(&material_path) {
                // ron::ser::to_writer_pretty(f, &materials, Default::default()).expect("failed");
                ron::de::from_reader(f).expect("failed to read existing material file")
            } else {
                HashMap::new()
            };

            println!("category: {}", category_name);

            for (material_name, ent) in std::fs::read_dir(ent.path())
                .expect("read_dir (level2) failed")
                .filter_map(dir_filter)
            {
                let output_dir = args
                    .asset_dir
                    .join("images")
                    // .join(&category_name)
                    .join(&material_name);
                if output_dir.exists() {
                    continue;
                }
                println!("{} {:?}", material_name, ent.path());

                let images = read_images(ent.path());
                println!(" {:?}", images.keys());

                std::fs::create_dir_all(&output_dir).expect("create_dir failed");

                if true {
                    let material = write_material_images(images, &material_name, output_dir);
                    materials.insert(
                        format!("material/{}/{}", category_name, material_name),
                        material,
                    );
                }
            }

            std::fs::create_dir_all(&materials_dir).expect("create_dir failed");
            if !materials.is_empty() {
                if let Ok(f) = std::fs::File::create(material_path) {
                    ron::ser::to_writer_pretty(f, &materials, Default::default()).expect("failed");
                }
            }
        }
    }
}

fn import_single_material(
    input_pbr: impl AsRef<Path>,
    name: &Option<String>,
    asset_dir: impl AsRef<Path>,
    existing: bool,
) {
    let input_pbr = input_pbr.as_ref();
    let asset_dir = asset_dir.as_ref();
    let name = match name {
        Some(name) => name.to_string(),
        None => {
            let guessed_name = guess_name(input_pbr);
            let input = dialoguer::Input::new()
                .with_prompt("material name:")
                .with_initial_text(guessed_name)
                .interact_text();

            input.expect("failed to get material name")
        }
    };
    let output_dir = asset_dir.join("images").join(&name);
    if output_dir.exists() {
        println!("outdir exists: {:?}", output_dir);
        if !existing {
            std::process::exit(1)
        }
    } else {
        std::fs::create_dir_all(&output_dir)
            .unwrap_or_else(|_| panic!("create_dir failed: {:?}", output_dir));
    }
    if !output_dir.is_dir() {
        println!("outdir is not a directory: {:?}", output_dir);
        std::process::exit(1);
    }
    let material_name = asset_dir.join("materials").join(format!("{}.ron", name));
    if material_name.exists() {
        println!("material name exists {:?}", material_name);
        if !existing {
            std::process::exit(1)
        }
    }
    let images = read_images(input_pbr);
    let material = write_material_images(images, &name, output_dir);
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

fn read_images<A>(
    input_path: A,
) -> bevy::utils::hashbrown::HashMap<ImageKind, (String, DynamicImage)>
where
    A: AsRef<Path>,
{
    let input_path = input_path.as_ref();
    let mut images = HashMap::new();
    // let image = Vec::new();
    let is_zip = input_path.extension().map_or(false, |s| s == "zip");
    if is_zip {
        let zip_file = std::fs::File::open(input_path).expect("failed to open zip file");
        let mut zip_archive = ZipArchive::new(zip_file).expect("failed to open zip archive");
        let names = zip_archive
            .file_names()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        images.extend(names.iter().filter_map(|s| {
            let name = s.to_string();
            let kind = ImageKind::guess_kind(&name)?;
            Some((
                kind,
                (name.clone(), read_image_from_zip(&mut zip_archive, &name)),
            ))
        }));
        // println!("{:?}", file_names);
    } else {
        images.extend(
            std::fs::read_dir(input_path)
                .expect("failed to read dir")
                .filter_map(|filename| {
                    let entry = filename.ok()?;
                    if !entry.metadata().ok()?.is_file() {
                        return None;
                    }
                    let name = entry.file_name().to_str()?.to_string();
                    let kind = ImageKind::guess_kind(&name)?;

                    Some((
                        kind,
                        (name, read_image(std::fs::File::open(entry.path()).ok()?)),
                    ))
                }),
        );
    }
    images
}

fn write_material_images(
    mut images: bevy::utils::hashbrown::HashMap<ImageKind, (String, DynamicImage)>,
    name: &str,
    output_dir: PathBuf,
) -> material::Material {
    let (_, albedo_image) = images
        .remove(&ImageKind::Albedo)
        .expect("missing albedo image");
    let (_, normal_image) = images
        .remove(&ImageKind::Normal)
        .expect("missing normal image");
    let (_, roughness_image) = images
        .remove(&ImageKind::Roughness)
        .expect("missing roughness image");
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
    let rm_image = if let Some((_, metallic_image)) = images.remove(&ImageKind::Metallic) {
        println!("metallic image: {:?}", metallic_image.color());
        let m = metallic_image.into_luma8(); //.expect("as_rgb8 failed");
        let r = roughness_image.into_luma8(); // .expect("as_rgb8 failed");
        ImageBuffer::from_fn(albedo_image.width(), albedo_image.height(), |x, y| {
            Rgb::<u8>([0, r.get_pixel(x, y).0[0], m.get_pixel(x, y).0[0]])
        })
    } else {
        roughness_image.into_rgb8()
    };
    let albedo_output = format!("{}_albedo.png", name);
    let normal_output = format!("{}_normal.norm", name);
    let ao_output = format!("{}_ao.norm", name);
    let mr_output = format!("{}_mr.norm", name);
    let emissive_output = format!("{}_emissive.norm", name);
    albedo_image
        .into_rgb8()
        .save_with_format(output_dir.join(&albedo_output), image::ImageFormat::Png)
        .expect("failed tp write albedo image");
    normal_image
        .into_rgb8()
        .save_with_format(output_dir.join(&normal_output), image::ImageFormat::Png)
        .expect("failed tp write normal image");
    rm_image
        .save_with_format(output_dir.join(&mr_output), image::ImageFormat::Png)
        .expect("failed tp rm albedo image");
    if let Some((ao, ao_image)) = images.remove(&ImageKind::Ao) {
        println!("ao image: {:?}", ao_image.color());

        let ao_image = ao_image.into_luma8();

        ao_image
            .save_with_format(output_dir.join(&ao_output), image::ImageFormat::Png)
            .expect("failed tp write ao image");
    }
    if let Some((_, emissive_image)) = images.remove(&ImageKind::Emissive) {
        emissive_image
            .into_rgb8()
            .save_with_format(output_dir.join(&emissive_output), image::ImageFormat::Png)
            .expect("failed to write emissive image");
    }
    material::Material {
        base: Some(format!("images/{}/{}", name, albedo_output)),
        occlusion: if images.contains_key(&ImageKind::Ao) {
            Some(format!("images/{}/{}", name, ao_output))
        } else {
            None
        },
        normal_map: Some(format!("images/{}/{}", name, normal_output)),
        metallic_roughness_texture: Some(format!("images/{}/{}", name, mr_output)),
        metallic: Some(1.0),
        roughness: Some(1.0),
        base_color: None,
        emissive: if images.contains_key(&ImageKind::Emissive) {
            Some(format!("images/{}/{}", name, emissive_output))
        } else {
            None
        },
        emissive_color: if images.contains_key(&ImageKind::Emissive) {
            Some(Vec3::ONE)
        } else {
            None
        },
        reflectance: None,
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

fn guess_name(pbr_zip: impl AsRef<Path>) -> String {
    let prefix = pbr_zip
        .as_ref()
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
