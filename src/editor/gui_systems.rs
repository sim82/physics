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
) {
    let materials_res = &mut *materials_res;

    let mut appearance_clicked = None;
    let mut material_clicked = None;
    let start = Instant::now();
    egui::Window::new("material browser")
        .open(&mut material_browser.window_open)
        .vscroll(true)
        .show(egui_context.ctx_mut(), |ui| {
            for app in materials_res.symlinks.keys() {
                if ui.button(app).clicked() {
                    appearance_clicked = Some(app);
                }
            }

            let mut iter = materials_res.material_defs.keys().peekable();
            while let Some(first) = iter.peek() {
                if let Some((section, _)) = first.rsplit_once('/') {
                    let mut entries = Vec::new();
                    while let Some(cur) = iter.peek() {
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
                                for (short_name, full_name) in entries {
                                    if ui.button(short_name).clicked() {
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
}
