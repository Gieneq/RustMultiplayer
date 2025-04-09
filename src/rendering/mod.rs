use crate::game::common::Vector2F;

pub mod renderer;

#[derive(Debug, Copy, Clone)]
pub struct EntityView {
    pub position: Vector2F,
    pub size: Vector2F,
    pub color: [f32; 3]
}



#[derive(Default)]
pub struct AppData {
    pub entities: Vec<EntityView>,
    pub camera_position: Vector2F,
    pub scale: f32,
}