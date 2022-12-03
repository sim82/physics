use bevy::{prelude::*, utils::Instant};
use bevy_egui::EguiContext;
use bevy_inspector_egui::egui;

use crate::AppState;

use super::resources;
// use bevy_

pub fn materials_egui_system(
    mut egui_context: ResMut<EguiContext>,
    mut materials_res: ResMut<resources::Materials>,
    mut material_browser: ResMut<resources::MaterialBrowser>,
    // images: Res<Assets<Image>>,
    mut asset_server: ResMut<AssetServer>,
    // mut cached_sections: Local<
) {
    let materials_res = &mut *materials_res;
    let material_browser = &mut *material_browser;

    let mut appearance_clicked = None;
    let mut material_clicked = None;
    let start = Instant::now();
    let mut window_open = material_browser.window_open;
    egui::Window::new("material browser")
        .open(&mut window_open)
        .vscroll(true)
        .show(egui_context.ctx_mut(), |ui| {
            for app in materials_res.symlinks.keys() {
                if ui.button(app).clicked() {
                    appearance_clicked = Some(app);
                }
            }

            // material browser ui: generate section headers on the fly by scanning runs with equal prefix in (sorted) material map.
            // probably not the most efficient thing in the world but good enough since it is only done when the ui is shown, and
            // this way there is no kind of caching or preprocessing that might get out of sync...
            let mut iter = materials_res.material_defs.iter().peekable();
            let mut entries = Vec::new();
            while let Some((first, _)) = iter.peek() {
                if let Some((section, _)) = first.rsplit_once('/') {
                    // extract elements with the same prefix. re-using temporary vector
                    entries.clear();
                    while let Some((cur, _)) = iter.peek() {
                        let (_mat_section, mat_name) =
                            if let Some((mat_section, mat_name)) = cur.rsplit_once('/') {
                                if mat_section != section {
                                    break;
                                }
                                (mat_section, mat_name)
                            } else {
                                break;
                            };

                        entries.push((mat_name, iter.next().unwrap()));
                    }

                    egui::CollapsingHeader::new(section)
                        .default_open(false)
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                for (_short_name, (full_name, material)) in &entries {
                                    if ui
                                        .add(egui::ImageButton::new(
                                            material_browser
                                                .get_preview(material)
                                                .expect("missing preview"),
                                            egui::Vec2::splat(64.0),
                                        ))
                                        .clicked()
                                    {
                                        material_clicked = Some(*full_name);
                                    }
                                }
                            });
                        });
                } else {
                    info!("no section in material name: {}", first);
                    iter.next();
                    continue;
                }
            }
        });

    debug!("dt: {:?}", start.elapsed());

    if let Some(clicked) = appearance_clicked {
        material_browser.selected_appearance = clicked.clone();
    }
    if let Some(clicked) = material_clicked {
        info!("clicked: {}", clicked);
        materials_res.update_symlink(
            material_browser.selected_appearance.clone(),
            clicked.clone(),
        );
    }
    material_browser.window_open = window_open;
}
