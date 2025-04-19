use crate::app::client::gui_client::AppData;

use super::GuiLayout;
use std::{cell::RefCell, rc::Rc};
#[derive(Debug)]
pub struct IngameGuiLayout {
    pub app_data: Rc<RefCell<AppData>>
}
const SCROLL_SENSITIVITY: f32 = 0.1;

impl GuiLayout for IngameGuiLayout {
    fn new(app_data: Rc<RefCell<AppData>>) -> Self {
        Self { app_data }
    }
}



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
