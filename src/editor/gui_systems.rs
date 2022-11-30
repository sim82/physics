use bevy::prelude::*;
use bevy_egui::EguiContext;
use bevy_inspector_egui::egui;

use super::resources;
// use bevy_

pub fn materials_egui_system(
    mut egui_context: ResMut<EguiContext>,
    mut materials_res: ResMut<resources::Materials>,
) {
    let materials_res = &mut *materials_res;
    egui::Window::new("materials").show(egui_context.ctx_mut(), |ui| {
        egui::Grid::new("my_grid").num_columns(2).show(ui, |ui| {
            for (appearance, material) in &mut materials_res.symlinks {
                ui.label(appearance);
                let mut selected = material.clone();
                egui::ComboBox::from_label(appearance).show_ui(ui, |ui| {
                    for (name, _) in &materials_res.material_defs {
                        ui.selectable_value(&mut selected, name.clone(), name);
                    }
                });
                if selected != *material {
                    info!("changed: {} {}", appearance, material);
                    *material = selected;
                    materials_res.dirty = true;
                }
                // ui.add(combo_box);
                ui.end_row();
            }
        });
    });
}
