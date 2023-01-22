#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum AppState {
    // DebugMenu,
    InGame,
    Editor,
    // Paused,
}
pub mod render_layers {
    use bevy::render::view::{Layer, RenderLayers};

    pub const MAIN_3D: Layer = 0;
    pub const TOP_2D: Layer = 1;
    pub const SIDE_2D: Layer = 2;
    pub fn ortho_views() -> RenderLayers {
        RenderLayers::from_layers(&[TOP_2D, SIDE_2D])
    }
}