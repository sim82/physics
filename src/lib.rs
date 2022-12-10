use bevy::{app::AppExit, prelude::*};

pub mod contact_debug;
// pub mod debug_lines;
pub mod appearance;
pub mod csg;
pub mod editor;
pub mod sky;
pub mod slidemove;
pub mod trace;
pub mod wsx;
pub mod sstree;
pub mod norm {
    // srgb workaround from https://github.com/bevyengine/bevy/issues/6371
    use bevy::asset::{AssetLoader, Error, LoadContext, LoadedAsset};
    use bevy::render::texture::{CompressedImageFormats, Image, ImageType};
    use bevy::utils::BoxedFuture;

    #[derive(Default)]
    pub struct NormalMappedImageTextureLoader;

    impl AssetLoader for NormalMappedImageTextureLoader {
        fn load<'a>(
            &'a self,
            bytes: &'a [u8],
            load_context: &'a mut LoadContext,
        ) -> BoxedFuture<'a, Result<(), Error>> {
            Box::pin(async move {
                let dyn_img = Image::from_buffer(
                    bytes,
                    ImageType::Extension("png"),
                    CompressedImageFormats::all(),
                    false,
                )
                .unwrap();

                load_context.set_default_asset(LoadedAsset::new(dyn_img));
                Ok(())
            })
        }

        fn extensions(&self) -> &[&str] {
            &["norm"]
        }
    }
}

pub mod material;

pub const OVERCLIP: f32 = 1.001;

pub mod test_texture {
    pub const TW: usize = 256;
    pub const TH: usize = 256;

    pub fn create() -> Vec<u8> {
        // let mut bitmap = [0u32; TW * TH];

        let mut bitmap = Vec::new();

        for y in 0..TH as i32 {
            for x in 0..TW as i32 {
                let l = (0x1FF
                    >> [x, y, TW as i32 - 1 - x, TH as i32 - 1 - y, 31]
                        .iter()
                        .min()
                        .unwrap()) as i32;

                let d = std::cmp::min(
                    50,
                    std::cmp::max(
                        0,
                        255 - 50
                            * f32::powf(
                                f32::hypot(
                                    x as f32 / (TW / 2) as f32 - 1.0f32,
                                    y as f32 / (TH / 2) as f32 - 1.0f32,
                                ) * 4.0,
                                2.0f32,
                            ) as i32,
                    ),
                );
                let r = (!x & !y) & 255;
                let g = (x & !y) & 255;
                let b = (!x & y) & 255;
                bitmap.extend(
                    [
                        (l.max(r - d)).clamp(0, 255) as u8,
                        (l.max(g - d)).clamp(0, 255) as u8,
                        (l.max(b - d)).clamp(0, 255) as u8,
                        0u8,
                    ]
                    .iter(),
                );
            }
        }
        bitmap
    }
}

#[derive(Resource, Default)]
pub struct TestResources {
    pub uv_image: Handle<Image>,
    pub uv_material: Handle<StandardMaterial>,
}

pub mod player_controller;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum AppState {
    DebugMenu,
    InGame,
    // Paused,
}

pub fn exit_on_esc_system(
    keyboard_input: Res<Input<KeyCode>>,
    mut app_exit_events: EventWriter<AppExit>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        app_exit_events.send_default();
    }
}
