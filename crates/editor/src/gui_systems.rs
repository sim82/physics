use bevy::{prelude::*, utils::Instant};
use bevy_egui::EguiContexts;
use bevy_inspector_egui::egui;

use super::resources;
// use bevy_

pub fn materials_egui_system(
    mut egui_context: EguiContexts,
    mut materials_res: ResMut<resources::Materials>,
    mut material_browser: ResMut<resources::MaterialBrowser>,
    // images: Res<Assets<Image>>,
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
            (appearance_clicked, material_clicked) =
                material_browser_ui(materials_res, ui, &mut *material_browser, None);
        });

    debug!("dt: {:?}", start.elapsed());

    if let Some(clicked) = appearance_clicked {
        material_browser.selected_appearance = clicked;
    }
    if let Some(clicked) = material_clicked {
        info!("clicked: {}", clicked);
        materials_res.update_symlink(material_browser.selected_appearance.clone(), clicked);
    }
    material_browser.window_open = window_open;
}

pub fn material_browser_ui(
    materials_res: &mut resources::Materials,
    ui: &mut egui::Ui,
    // appearance_clicked: &mut Option<&String>,
    material_browser: &mut resources::MaterialBrowser,
    // material_clicked: &mut Option<&String>,
    max_width: Option<f32>,
) -> (Option<String>, Option<String>) {
    let mut appearance_clicked = None;
    let mut material_clicked = None;

    for app in materials_res.symlinks.keys() {
        if ui.button(app).clicked() {
            appearance_clicked = Some(app.clone());
        }
    }
    // material browser ui: generate section headers on the fly by scanning runs with equal prefix in (sorted) material map.
    // probably not the most efficient thing in the world but good enough since it is only done when the ui is shown, and
    // this way there is no kind of caching or preprocessing that might get out of sync...
    let mut iter = materials_res.material_defs.iter().peekable();
    while let Some((first, _)) = iter.peek() {
        if let Some((section, _)) = first.rsplit_once('/') {
            let mut used = false;
            egui::CollapsingHeader::new(section)
                .default_open(false)
                .show(ui, |ui| {
                    if let Some(width) = max_width {
                        ui.set_max_width(width);
                    }

                    ui.horizontal_wrapped(|ui| {
                        used = true;
                        // scan through run with equal prefix, adding ImageButtons on the fly.
                        while let Some((cur, material)) = iter.peek() {
                            let material_name = match cur.rsplit_once('/') {
                                Some((mat_section, _)) if mat_section != section => break, // end of run
                                None => break, // no prefix -> ignore
                                Some((_, name)) => name,
                            };
                            if let Some(preview_image) = material_browser.get_preview(material) {
                                if ui
                                    .add(egui::ImageButton::new(
                                        preview_image,
                                        egui::Vec2::splat(64.0),
                                    ))
                                    .on_hover_text(material_name)
                                    .clicked()
                                {
                                    material_clicked = Some((*cur).clone());
                                }
                            }
                            let _ = iter.next();
                        }
                    });
                });
            if !used {
                // if the run was not consumed (i.e. the header was collapsed) skip over them in iterator.
                while let Some((cur, _material)) = iter.peek() {
                    match cur.rsplit_once('/') {
                        Some((mat_section, _)) if mat_section != section => break,
                        None => break,
                        _ => {
                            let _ = iter.next();
                        }
                    };
                }
            }
        } else {
            info!("no section in material name: {}", first);
            iter.next();
            continue;
        }
    }
    (appearance_clicked, material_clicked)
}
