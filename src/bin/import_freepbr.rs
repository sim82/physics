use std::{
    io::{BufReader, Cursor, Read},
    path::{Path, PathBuf},
};

use clap::Parser;
use image::{DynamicImage, ImageBuffer, Rgb};
use log::{info, warn};
use zip::read::ZipArchive;

#[derive(clap::Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct CmdlineArgs {
    pbr_zip: PathBuf,
}

fn main() {
    env_logger::init();
    let args = CmdlineArgs::parse();
    println!("{:?}", args.pbr_zip);

    // let Some(pbr_zip) = args.pbr_zip else {

    // }

    let zip_file = std::fs::File::open(args.pbr_zip).expect("failed to open zip file");
    let mut zip_archive = ZipArchive::new(zip_file).expect("failed to open zip archive");

    // let file_paths = zip_archive
    //     .file_names()
    //     .map(|x| x.into())
    //     .collect::<Vec<PathBuf>>();

    let file_names = zip_archive
        .file_names()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    println!("{:?}", file_names);
    let mut albedo = None;
    let mut ao = None;
    let mut metallic = None;
    let mut roughness = None;
    let mut normal = None;

    for name in &file_names {
        let lower = name.to_lowercase();

        if lower.contains("albedo") {
            albedo = Some(name);
        } else if lower.contains("ao") {
            ao = Some(name);
        } else if lower.contains("metallic") {
            metallic = Some(name);
        } else if lower.contains("roughness") {
            roughness = Some(name);
        } else if lower.contains("normal") {
            if !lower.contains("ogl") {
                warn!("not 'ogl' in normal map. Check handedness.: {}", name);
            }
            normal = Some(name)
        }
    }

    let albedo_image = read_image(&mut zip_archive, albedo.expect("missing albedo image"));
    println!("albedo image: {:?}", albedo_image.color());
    let normal_image = read_image(&mut zip_archive, normal.expect("missing normal image"));
    println!("normal image: {:?}", normal_image.color());
    let ao_image = read_image(&mut zip_archive, ao.expect("missing ao image"));
    println!("ao image: {:?}", ao_image.color());
    let roughness_image = read_image(
        &mut zip_archive,
        roughness.expect("missing roughness image"),
    );
    println!("roughness image: {:?}", roughness_image.color());
    let metallic_image = read_image(&mut zip_archive, metallic.expect("missing metallic image"));
    println!("metallic image: {:?}", metallic_image.color());
    let m = metallic_image.into_luma8(); //.expect("as_rgb8 failed");
    let r = roughness_image.into_luma8(); // .expect("as_rgb8 failed");
    let rm_image = ImageBuffer::from_fn(albedo_image.width(), albedo_image.height(), |x, y| {
        Rgb::<u8>([0, r.get_pixel(x, y).0[0], m.get_pixel(x, y).0[0]])
    });
    rm_image.save("test.png").unwrap();
}

fn read_image(zip_archive: &mut ZipArchive<std::fs::File>, name: &String) -> image::DynamicImage {
    let mut albedo_zip = zip_archive
        .by_name(name)
        .unwrap_or_else(|_| panic!("failed to get image zip entry: {}", name));
    let mut data = Vec::new();
    albedo_zip.read_to_end(&mut data).expect("read failed");

    image::load(
        Cursor::new(&data),
        image::ImageFormat::from_path(name).expect("failed to guess image type"),
    )
    .expect("image load failed")
}
