use bevy::prelude::*;
use bevy_egui::EguiContext;
use bevy_inspector_egui::egui;
use bevy_rapier3d::render::DebugRenderContext;

use crate::editor;

fn debug_gui_system(
    mut egui_context: ResMut<EguiContext>,
    rapier_debug_context: Option<ResMut<DebugRenderContext>>,
    material_browser: ResMut<editor::resources::MaterialBrowser>, // FIXME: handle window state properly
) {
    let mut open = material_browser.window_open;
    egui::Window::new("debug controls")
        .open(&mut open)
        .show(egui_context.ctx_mut(), |ui| {
            if let Some(mut rapier_debug_context) = rapier_debug_context {
                ui.group(|ui| {
                    ui.label("rapier");
                    ui.checkbox(&mut rapier_debug_context.enabled, "show");
                    ui.checkbox(&mut rapier_debug_context.always_on_top, "on top");
                });
            }
        });
}

pub struct DebugGuiPlugin;

impl Plugin for DebugGuiPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(debug_gui_system);
    }
}
