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
) {
    let wm_state = &mut *wm_state;

    egui::Window::new("main 3d").show(egui_context.ctx_mut(), |ui| {
        ui.image(
            wm_state.slot_main3d.offscreen_egui_texture,
            egui::Vec2::new(512.0, 512.0),
        );
    });

    egui::Window::new("2d").show(egui_context.ctx_mut(), |ui| {
        let size_upper = egui::Vec2::new(
            ui.available_width(),
            ui.available_height() / 2.0 - 32.0 + wm_state.separator_bias,
        );
        let size_lower = egui::Vec2::new(
            ui.available_width(),
            ui.available_height() / 2.0 - 32.0 - wm_state.separator_bias,
        );

        show_2d_view(
            ui,
            &mut wm_state.slot_upper2d,
            &mut event_writer,
            resources::UPPER_WINDOW,
            size_upper,
        );

        // TODO: somehow make the separator draggable

        show_2d_view(
            ui,
            &mut wm_state.slot_lower2d,
            &mut event_writer,
            resources::LOWER_WINDOW,
            size_lower,
        );

        let zoom_delta = ui.input().zoom_delta();
        if zoom_delta != 1.0 {
            event_writer.send(WmEvent::ZoomDelta(zoom_delta)); // uhm yeah, why not...
        }

        // wm_state.separator_bias += response.drag_delta().y;
    });

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
