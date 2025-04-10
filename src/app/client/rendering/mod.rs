use crate::game::math::Vector2F;

pub mod renderer;

#[derive(Debug, Copy, Clone)]
pub struct EntityView {
    pub position: Vector2F,
    pub size: Vector2F,
    pub color: [f32; 3]
}

// TODO rename to something rendering related
#[derive(Default)]
pub struct AppData {
    pub entities: Vec<EntityView>,
    pub camera_position: Vector2F,
    pub scale: f32,
}
