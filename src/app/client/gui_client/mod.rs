pub mod renderer;
pub mod guis;

use clap::builder::styling::RgbColor;
use guis::AppGuiTransition;

use crate::game::math::{Rect2F, Vector2F};

use super::MultiplayerClientHandle;

#[derive(Debug, Copy, Clone)]
pub struct EntityView {
    pub rect: Rect2F,
    pub color: RgbColor,
    pub marker_color: Option<RgbColor>,
}

#[derive(Debug)]
pub struct AppData {
    pub client_handler: Option<MultiplayerClientHandle>,
    pub player_name: Option<String>,
    pub app_gui_expected_transition: Option<AppGuiTransition>,
    pub last_width: f32,
    pub last_height: f32,
    pub world_scale: f32,
    pub camera: Vector2F,
}



        
