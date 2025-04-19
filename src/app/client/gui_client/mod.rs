pub mod renderer;
pub mod gui_renderer;
pub mod guis;

use guis::AppGui;
use renderer::Renderer;
use winit::event::{KeyEvent, MouseScrollDelta};

use crate::game::math::Vector2F;

#[derive(Debug, Copy, Clone)]
pub struct EntityView {
    pub position: Vector2F,
    pub size: Vector2F,
    pub color: [f32; 3]
}

// TODO rename to something rendering related
pub struct AppData {
    pub active_app_gui: AppGui
}


// impl Default for AppStageView {
//     fn default() -> Self {
//         Self::Disconnected { gui: DisconnectedGuiLayout {  } }
//     }
// }

#[derive(Debug)]
pub struct AppIngameStageData {
    pub entities: Vec<EntityView>,
    pub camera_position: Vector2F,
    pub scale: f32,
}

// impl AppStageView {
//     pub fn process_key_event(&mut self, event: KeyEvent) {
 
//     }

//     pub fn process_mouse_wheele(&mut self, delta: MouseScrollDelta) {
//     }

//     pub fn update(&mut self, dt: std::time::Duration) {
//         println!("dt={}", dt.as_millis());
//     }

//     pub fn draw(&self, render_state: &mut State) {
//     }
// }








        // if event.state == ElementState::Released {
        //     let client_handler = self.client_handler.as_ref().unwrap();
        //     match event.logical_key {
        //         Key::Named(winit::keyboard::NamedKey::ArrowUp) => {
        //             // self.client_handler.as_ref().unwrap().move_headless(MoveDirection::Up);
        //             let _ = client_handler.make_request(ClientRequest::Move { dir: MoveDirection::Up} );
        //         },
        //         Key::Named(winit::keyboard::NamedKey::ArrowRight) => {
        //             // self.client_handler.as_ref().unwrap().move_headless(MoveDirection::Right);
        //             let _ = client_handler.make_request(ClientRequest::Move { dir: MoveDirection::Right} );
        //         },
        //         Key::Named(winit::keyboard::NamedKey::ArrowDown) => {
        //             // self.client_handler.as_ref().unwrap().move_headless(MoveDirection::Down);
        //             let _ = client_handler.make_request(ClientRequest::Move { dir: MoveDirection::Down} );
        //         },
        //         Key::Named(winit::keyboard::NamedKey::ArrowLeft) => {
        //             // self.client_handler.as_ref().unwrap().move_headless(MoveDirection::Left);
        //             let _ = client_handler.make_request(ClientRequest::Move { dir: MoveDirection::Left} );
        //         },
        //         _ => {}
        //     }
        // } 

        // match delta {
        //     winit::event::MouseScrollDelta::LineDelta(_, y) => {
        //         // y is +-1
        //         if let Ok(mut app_data_guard) = self.data.lock() {
        //             app_data_guard.scale *= (1.0 + SCROLL_SENSITIVITY).powf(y);
        //         }
        //     },
        //     winit::event::MouseScrollDelta::PixelDelta(_physical_position) => todo!(),
        // }


        



                    // if let Ok(app_data_guard) = self.data.lock() {
                    //     state.render(&app_data_guard)
                    // };