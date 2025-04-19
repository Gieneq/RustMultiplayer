pub mod renderer;
pub mod guis;

use guis::AppGuiTransition;

use crate::game::math::Vector2F;

use super::MultiplayerClientHandle;

#[derive(Debug, Copy, Clone)]
pub struct EntityView {
    pub position: Vector2F,
    pub size: Vector2F,
    pub color: [f32; 3]
}

#[derive(Debug)]
pub struct AppData {
    pub client_handler: Option<MultiplayerClientHandle>,
    pub player_name: Option<String>,
    pub app_gui_expected_transition: Option<AppGuiTransition>,
    pub last_width: f32,
    pub last_height: f32,
}



        
