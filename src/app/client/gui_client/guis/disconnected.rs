use std::{
    cell::RefCell, 
    rc::Rc
};

use winit::{
    dpi::PhysicalPosition, 
    event::{
        ElementState, 
        MouseButton
    }
};

use crate::{
    app::client::gui_client::{
        guis::{
            components::templates::{
                build_gui_plain_button, 
                GuiComponentSize
            }, 
            AppGuiTransition
        }, 
        renderer::Renderer, 
        AppData
    }, 
    game::math::Vector2F
};

use super::{
    components::GuiPlainButton, 
    GuiElement, 
    GuiLayout
};

#[derive(Debug)]
pub struct DisconnectedGuiLayout {
    proceed_button: GuiPlainButton,
    pub app_data: Rc<RefCell<AppData>>
}

impl GuiLayout for DisconnectedGuiLayout {
    fn new(app_data: Rc<RefCell<AppData>>) -> Self {
        log::info!("Entered 'Disconneted' gui");
        
        let (width, height) = {
            let app_data_borrowed = app_data.borrow();
            (app_data_borrowed.last_width, app_data_borrowed.last_height)
        };

        let mut result = Self {
            proceed_button: build_gui_plain_button(
                Vector2F::zero(), 
                GuiComponentSize::Big
            ),
            app_data,
        };
        result.resize_window(width, height);
        result
    }

    fn resize_window(&mut self, width: f32, height: f32) {
        self.proceed_button.rect.pos.x = (width - self.proceed_button.rect.size.x) / 2.0;
        self.proceed_button.rect.pos.y = (height - self.proceed_button.rect.size.y) / 2.0;
    }
    
    fn process_mouse_events(&mut self, position: PhysicalPosition<f64>, button_state: ElementState, button: MouseButton) { 
        if button_state == ElementState::Released && button == MouseButton::Left {
            let mouse_pos = Vector2F::new(position.x as f32, position.y as f32);
            if self.proceed_button.is_inside(&mouse_pos) {
                log::info!("Button clicked");
                let response = {
                    let app_data = self.app_data.borrow();
                    let new_name = app_data.player_name.clone();
                    let cleint_handle = app_data.client_handler.as_ref().unwrap();
                    cleint_handle.make_request(crate::requests::ClientRequest::SetName { new_name }).unwrap()
                };

                let was_set = match response {
                    crate::requests::ClientResponse::SetName { result } => result.is_ok(),
                    _ => false,
                };

                if was_set {
                    log::info!("Name was set, can proceed");
                    let mut app_data = self.app_data.borrow_mut();
                    app_data.app_gui_expected_transition = Some(AppGuiTransition::ToLobby);
                } else {
                    log::warn!("Could not set name");
                }
            }
        }
    }

    fn draw(&self, renderer: &mut Renderer) {
        let (outer_gui_box, inner_gui_box) = self.proceed_button.get_drawable_rects();
        renderer.batch_append_gui_element(GuiElement::Box(outer_gui_box));
        renderer.batch_append_gui_element(GuiElement::Box(inner_gui_box));
    }
}