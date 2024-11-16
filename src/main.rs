use bevy::{prelude::*, render::texture::ImageSamplerDescriptor};
use editor::EditorPluginGroup;
use physics::{ExternalPluginGroup, GamePluginGroup};

fn main() {
    let mut app = App::new();

    // app.add_plugins(DefaultPlugins);
    app.add_plugins(DefaultPlugins.set(ImagePlugin {
        default_sampler: ImageSamplerDescriptor {
            // mipmap_filter: bevy::render::texture::ImageFilterMode::Linear,
            min_filter: bevy::render::texture::ImageFilterMode::Linear,
            mag_filter: bevy::render::texture::ImageFilterMode::Linear,
            // mag_filter: wgpu::FilterMode::Linear,
            // min_filter: wgpu::FilterMode::Linear,
            // mipmap_filter: wgpu::FilterMode::Linear,
            address_mode_u: bevy::render::texture::ImageAddressMode::Repeat,
            address_mode_v: bevy::render::texture::ImageAddressMode::Repeat,
            address_mode_w: bevy::render::texture::ImageAddressMode::Repeat,
            ..Default::default()
        },
    }));

    // app.add_systems(Update, bevy::window::close_on_esc);

    app.add_plugins(GamePluginGroup);
    app.add_plugins(EditorPluginGroup);
    app.add_plugins(ExternalPluginGroup);
    app.run();

    info!("after app.run");
}
