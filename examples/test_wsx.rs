use std::io::BufReader;

use physics::{csg, wsx};

fn main() {
    let file = std::fs::File::open("nav3.wsx").unwrap();
    println!("res");
    let wsx: wsx::WiredExportScene = quick_xml::de::from_reader(BufReader::new(file)).unwrap();
    println!("wsx: {:?}", wsx);

    for node in &wsx.SceneNodes.SceneNode {
        let Some(brushes) = &node.Components.Brush else {continue};
        for brush in brushes {
            let csg_brush: csg::Brush = brush.into();

            println!("{:?}", csg_brush);
        }
    }
}
