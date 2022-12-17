use bevy::{
    prelude::*,
    render::{
        camera::RenderTarget,
        view::{self, RenderLayers},
    },
};
use bevy_atmosphere::prelude::AtmosphereCamera;
use bevy_egui::EguiContext;
use bevy_inspector_egui::egui;
use wgpu::Extent3d;

use crate::{editor::util::WmMouseButton, player_controller::PlayerCamera, render_layers};

use super::{
    resources::{self, WmSlot},
    util::WmEvent,
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
) {
    let wm_state = &mut *wm_state;

    egui::Window::new("main 3d").show(egui_context.ctx_mut(), |ui| {
        ui.image(
            wm_state.slot_main3d.offscreen_egui_texture,
            egui::Vec2::new(512.0, 512.0),
        );
    });

    egui::Window::new("2d").show(egui_context.ctx_mut(), |ui| {
        let views = [
            (resources::UPPER_WINDOW, &mut wm_state.slot_upper2d),
            (resources::LOWER_WINDOW, &mut wm_state.slot_lower2d),
        ];

        for (name, slot) in views {
            egui::Resize::default()
                .id_source(name)
                // .default_size([150.0, 200.0])
                .show(ui, |ui| {
                    let size = ui.available_size();
                    slot.target_size = size;
                    // ui.image(slot.offscreen_egui_texture, size)

                    let image = egui::Image::new(slot.offscreen_egui_texture, size)
                        .sense(egui::Sense::click_and_drag());
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

                    if let Some(mut pos) = response.interact_pointer_pos() {
                        pos.x -= response.rect.min.x;
                        pos.y -= response.rect.min.y;
                        // response.
                        if response.drag_started() {
                            // slot.drag_initial_button = button;
                            slot.drag_active = true;
                            event_writer.send(WmEvent::DragStart {
                                window: name,
                                button,
                                pos: Vec2::new(pos.x, pos.y),
                            });
                        } else if response.dragged() && slot.drag_active {
                            event_writer.send(WmEvent::DragUpdate {
                                window: name,
                                button, //: slot.drag_initial_button,
                                pos: Vec2::new(pos.x, pos.y),
                            });
                        } else if response.drag_released() && slot.drag_active {
                            slot.drag_active = false;
                            event_writer.send(WmEvent::DragEnd {
                                window: name,
                                button, //: wm_state.slot_upper2d.drag_initial_button,
                                pos: Vec2::new(pos.x, pos.y),
                            });
                        }

                        // info!("response: {:?}", response);
                    }
                });
        }
    });

    wm_state.slot_upper2d.check_resize(&mut image_assets);
    wm_state.slot_lower2d.check_resize(&mut image_assets);
}
