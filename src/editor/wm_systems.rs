use bevy::prelude::*;

use bevy_egui::EguiContext;
use bevy_inspector_egui::egui;
use bevy_rapier3d::render::DebugRenderContext;

use crate::editor::util::WmMouseButton;

use super::{
    gui_systems,
    resources::{self, WmSidpanelContent, WmSlot},
    util::{WmEvent, WmEventPointerState},
};

pub fn wm_test_setup_system(
    mut egui_context: ResMut<EguiContext>,
    mut wm_state: ResMut<resources::WmState>,
    mut image_assets: ResMut<Assets<Image>>,
) {
    wm_state.slot_upper2d = WmSlot::new(&mut image_assets, &mut egui_context);
    wm_state.slot_lower2d = WmSlot::new(&mut image_assets, &mut egui_context);
    wm_state.slot_main3d = WmSlot::new(&mut image_assets, &mut egui_context);
}

pub fn wm_test_system(
    mut egui_context: ResMut<EguiContext>,
    mut wm_state: ResMut<resources::WmState>,
    mut image_assets: ResMut<Assets<Image>>,
    mut event_writer: EventWriter<WmEvent>,
    mut materials_res: ResMut<resources::Materials>,
    mut material_browser: ResMut<resources::MaterialBrowser>,
    rapier_debug_context: Option<ResMut<DebugRenderContext>>,
) {
    let wm_state = &mut *wm_state;

    egui::SidePanel::left("left side panel")
        .resizable(true)
        .default_width(768.0)
        .show(egui_context.ctx_mut(), |ui| {
            ui.vertical(|ui| {
                // info!("avalable: {:?}", ui.available_width());
                let width = ui.available_width();
                let size = egui::Vec2::new(width, 512.0);
                ui.image(wm_state.slot_main3d.offscreen_egui_texture, size);

                if wm_state.slot_main3d.target_size != size {
                    wm_state.slot_main3d.target_size = size;
                }

                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut wm_state.sidepanel_content,
                        WmSidpanelContent::Material,
                        "Mat",
                    );
                    ui.selectable_value(
                        &mut wm_state.sidepanel_content,
                        WmSidpanelContent::Miscsettings,
                        "Misc",
                    );
                });

                match wm_state.sidepanel_content {
                    WmSidpanelContent::Material => {
                        egui::ScrollArea::vertical()
                            .id_source("material browser")
                            .show(ui, |ui| {
                                let (appearance_clicked, material_clicked) =
                                    gui_systems::material_browser_ui(
                                        &mut materials_res,
                                        ui,
                                        &mut material_browser,
                                        None,
                                        // Some(512.0),
                                    );

                                if let Some(clicked) = appearance_clicked {
                                    material_browser.selected_appearance = clicked;
                                }
                                if let Some(clicked) = material_clicked {
                                    info!("clicked: {}", clicked);
                                    materials_res.update_symlink(
                                        material_browser.selected_appearance.clone(),
                                        clicked,
                                    );
                                }
                            });
                    }
                    WmSidpanelContent::Miscsettings => {
                        if let Some(mut rapier_debug_context) = rapier_debug_context {
                            ui.group(|ui| {
                                ui.label("rapier");
                                ui.checkbox(&mut rapier_debug_context.enabled, "show");
                                ui.checkbox(&mut rapier_debug_context.always_on_top, "on top");
                            });
                        }
                    }
                }

                // ui.allocate_space(ui.available_size());
            });
        });
    egui::CentralPanel::default().show(egui_context.ctx_mut(), |ui| {
        egui::TopBottomPanel::top("top 3d view")
            .resizable(true)
            .min_height(32.0)
            .default_height(512.0)
            .show(ui.ctx(), |ui| {
                let size_upper = ui.available_size();
                show_2d_view(
                    ui,
                    &mut wm_state.slot_upper2d,
                    &mut event_writer,
                    resources::UPPER_WINDOW,
                    size_upper,
                );
            });

        egui::CentralPanel::default()
            // .min_height(32.0)
            .show(ui.ctx(), |ui| {
                ui.set_min_height(32.0);
                let size_lower = ui.available_size();

                show_2d_view(
                    ui,
                    &mut wm_state.slot_lower2d,
                    &mut event_writer,
                    resources::LOWER_WINDOW,
                    size_lower,
                );
            });

        let zoom_delta = ui.input().zoom_delta();
        if zoom_delta != 1.0 {
            event_writer.send(WmEvent::ZoomDelta(zoom_delta)); // uhm yeah, why not...
        }
        // wm_state.separator_bias += response.drag_delta().y;
    });
    wm_state.slot_main3d.check_resize(&mut image_assets);
    wm_state.slot_upper2d.check_resize(&mut image_assets);
    wm_state.slot_lower2d.check_resize(&mut image_assets);
}

fn show_2d_view(
    ui: &mut egui::Ui,
    slot: &mut WmSlot,
    event_writer: &mut EventWriter<WmEvent>,
    name: &'static str,
    size: egui::Vec2,
) {
    // let size = ui.available_size();
    // info!("size: {:?}", size);
    // ui.
    slot.target_size = size;
    // ui.image(slot.offscreen_egui_texture, size)
    let image =
        egui::Image::new(slot.offscreen_egui_texture, size).sense(egui::Sense::click_and_drag());
    let response = ui.add(image);
    let pointer_state = &response.ctx.input().pointer;
    let button = if pointer_state.button_down(egui::PointerButton::Primary) {
        WmMouseButton::Left
    } else if pointer_state.button_down(egui::PointerButton::Middle) {
        WmMouseButton::Middle
    } else if pointer_state.button_down(egui::PointerButton::Secondary) {
        WmMouseButton::Right
    } else {
        WmMouseButton::Left
    };
    if let Some(pos) = response.interact_pointer_pos() {
        let pointer_state = WmEventPointerState {
            pos: Vec2::new(pos.x, pos.y),
            bounds: Rect::new(
                response.rect.min.x,
                response.rect.min.y,
                response.rect.max.x,
                response.rect.max.y,
            ),
        };

        let drag_allowed = button == WmMouseButton::Middle
            || button == WmMouseButton::Right
            || response.ctx.input().modifiers.ctrl
            || slot.drag_active;

        if drag_allowed {
            if response.drag_started() {
                slot.drag_active = true;
                event_writer.send(WmEvent::DragStart {
                    window: name,
                    button,
                    pointer_state,
                });
            } else if response.dragged() && slot.drag_active {
                event_writer.send(WmEvent::DragUpdate {
                    window: name,
                    button,
                    pointer_state,
                });
            } else if response.drag_released() && slot.drag_active {
                slot.drag_active = false;
                event_writer.send(WmEvent::DragEnd {
                    window: name,
                    button,
                    pointer_state,
                });
            }
        } else if response.clicked() {
            event_writer.send(WmEvent::Clicked {
                window: name,
                button,
                pointer_state,
            });
        }
    }
}
