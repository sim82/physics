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

            let mut iter = materials_res.material_defs.iter().peekable();
            while let Some((first, _)) = iter.peek() {
                if let Some((section, _)) = first.rsplit_once('/') {
                    let mut entries = Vec::new();
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
                                for (short_name, (full_name, material)) in entries {
                                    if ui
                                        .add(egui::ImageButton::new(
                                            material_browser
                                                .get_preview(
                                                    material,
                                                    // &mut *asset_server,
                                                    // &mut *egui_context,
                                                )
                                                .expect("missing preview"),
                                            egui::Vec2::splat(64.0),
                                        ))
                                        .clicked()
                                    {
                                        material_clicked = Some(full_name);
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
