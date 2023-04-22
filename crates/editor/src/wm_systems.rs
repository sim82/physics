use bevy::{ecs::system::SystemState, prelude::*};

use bevy_egui::EguiContexts;
use bevy_inspector_egui::egui;
use bevy_rapier3d::render::DebugRenderContext;

use crate::util::WmMouseButton;

use super::{
    gui_systems,
    resources::{self, WmSettings, WmSidpanelContent, WmSlot},
    util::{WmEvent, WmEventPointerState, WmModifiers},
};

pub fn wm_test_setup_system(
    mut egui_context: EguiContexts,
    mut wm_state: ResMut<resources::WmState>,
    mut image_assets: ResMut<Assets<Image>>,
) {
    wm_state.slot_upper2d = WmSlot::new(&mut image_assets, &mut egui_context);
    wm_state.slot_lower2d = WmSlot::new(&mut image_assets, &mut egui_context);
    wm_state.slot_main3d = WmSlot::new(&mut image_assets, &mut egui_context);

    if let Ok(file) = std::fs::File::open("wm_settings.yaml") {
        // TODO: migration strategy
        wm_state.settings =
            serde_yaml::from_reader(file).expect("failed to deserialize wm settings");
    }
}

pub fn wm_test_system(world: &mut World) {
    let mut system_state: SystemState<(
        EguiContexts,
        EventWriter<WmEvent>,
        Option<ResMut<DebugRenderContext>>,
        ResMut<resources::WmState>,
        ResMut<Assets<Image>>,
        ResMut<resources::Materials>,
        ResMut<resources::MaterialBrowser>,
        ResMut<resources::MiscSettings>,
    )> = SystemState::new(world);
    let (
        mut egui_context,
        mut event_writer,
        rapier_debug_context,
        mut wm_state,
        mut image_assets,
        mut materials_res,
        mut material_browser,
        mut misc_settings,
    ) = system_state.get_mut(world);
    egui::SidePanel::left("left side panel")
        .resizable(true)
        .default_width(wm_state.settings.sidepanel_separator)
        .show(egui_context.ctx_mut(), |ui| {
            wm_state.settings.sidepanel_separator = ui.available_width();
            ui.vertical(|ui| {
                // info!("avalable: {:?}", ui.available_width());
                let width = ui.available_width().round();
                let size = egui::Vec2::new(width, width / 1.6);

                let image = egui::Image::new(wm_state.slot_main3d.offscreen_egui_texture, size)
                    .sense(egui::Sense::click());
                let response = ui.add(image);

                send_wm_events_for_egui_response(
                    response,
                    &mut wm_state.slot_main3d,
                    &mut event_writer,
                    resources::MAIN3D_WINDOW,
                );

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
                    ui.selectable_value(
                        &mut wm_state.sidepanel_content,
                        WmSidpanelContent::Entities,
                        "Ent",
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
                                        clicked.clone(),
                                    );
                                    material_browser.selected_material = clicked;
                                }
                            });
                    }
                    WmSidpanelContent::Miscsettings => {
                        if let Some(mut rapier_debug_context) = rapier_debug_context {
                            ui.group(|ui| {
                                ui.label("rapier");
                                ui.checkbox(&mut rapier_debug_context.enabled, "show");
                                ui.checkbox(&mut rapier_debug_context.always_on_top, "on top");
                                ui.checkbox(
                                    &mut misc_settings.csg_wireframe,
                                    "csg output wireframe",
                                );
                                ui.checkbox(
                                    &mut misc_settings.csg_reverse_check,
                                    "csg reverse check",
                                );
                            });
                        }
                    }
                    WmSidpanelContent::Entities => { // meh, it is not really possible to integrate the world inspector here...
                    }
                }

                // ui.allocate_space(ui.available_size());
            });
        });
    egui::CentralPanel::default().show(egui_context.ctx_mut(), |ui| {
        egui::TopBottomPanel::top("top 2d view")
            .resizable(true)
            .min_height(32.0)
            .default_height(wm_state.settings.ortho_separator)
            .show(ui.ctx(), |ui| {
                wm_state.settings.ortho_separator = ui.available_height().round();
                // info!("size: {:?}", ui.available_size());
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

        let zoom_delta = ui.input(|i| i.zoom_delta());
        if zoom_delta != 1.0 {
            event_writer.send(WmEvent::ZoomDelta(zoom_delta)); // uhm yeah, why not...
        }
        // wm_state.separator_bias += response.drag_delta().y;
    });
    wm_state.slot_main3d.check_resize(&mut image_assets);
    wm_state.slot_upper2d.check_resize(&mut image_assets);
    wm_state.slot_lower2d.check_resize(&mut image_assets);

    #[cfg(feature = "inspector")]
    {
        // needs to be last due to exclusive world access
        egui::SidePanel::right("right side panel")
            .resizable(true)
            // .default_width(wm_state.settings.sidepanel_separator)
            .show(&egui_context.ctx_mut().clone(), |ui| {
                egui::ScrollArea::vertical()
                    .id_source("entity_browser")
                    .show(ui, |ui| {
                        bevy_inspector_egui::bevy_inspector::ui_for_world_entities(world, ui);
                    })
            });
    }
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
    send_wm_events_for_egui_response(response, slot, event_writer, name);
}

impl From<egui::Modifiers> for WmModifiers {
    fn from(modifiers: egui::Modifiers) -> Self {
        Self {
            shift: modifiers.shift,
            ctrl: modifiers.ctrl,
            alt: modifiers.alt,
        }
    }
}

fn send_wm_events_for_egui_response(
    response: egui::Response,
    slot: &mut WmSlot,
    event_writer: &mut EventWriter<WmEvent>,
    name: &'static str,
) {
    let pointer_state = response.ctx.input(|i| i.pointer.clone()); // FIXME: do all in one input block
    let modifiers = response.ctx.input(|i| i.modifiers.into());
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
            modifiers,
        };

        let drag_allowed = button == WmMouseButton::Middle
            || button == WmMouseButton::Right
            || response.ctx.input(|i| i.modifiers.ctrl)
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
                button: WmMouseButton::Left,
                pointer_state,
            });
        } else if response.secondary_clicked() {
            event_writer.send(WmEvent::Clicked {
                window: name,
                button: WmMouseButton::Right,
                pointer_state,
            });
        }
    }
}

pub fn write_view_settings(
    wm_state: Res<resources::WmState>,
    mut last_written_settings: Local<WmSettings>,
) {
    if wm_state.settings != *last_written_settings {
        if let Ok(file) = std::fs::File::create("wm_settings.yaml") {
            let _ = serde_yaml::to_writer(file, &wm_state.settings);
            *last_written_settings = wm_state.settings;
            info!("window settings written");
        }
    }
}
