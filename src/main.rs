use bevy::prelude::*;
use editor::EditorPluginGroup;
use physics::{ExternalPluginGroup, GamePluginGroup};

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(ImagePlugin {
        default_sampler: wgpu::SamplerDescriptor {
            // mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            ..Default::default()
        },
    }))
    .add_system(bevy::window::close_on_esc);

    app.add_plugins(GamePluginGroup);
    app.add_plugins(EditorPluginGroup);
    app.add_plugins(ExternalPluginGroup);
    app.run();

    info!("after app.run");
}
