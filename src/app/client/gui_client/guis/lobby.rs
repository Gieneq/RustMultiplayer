use winit::event::{
    ElementState, 
    MouseButton
};

use crate::{
    app::client::gui_client::{
        guis::components::templates::{
            build_gui_indicator, 
            build_gui_toggle_button, 
            GuiComponentSize
        }, 
        AppData
    }, 
    game::math::Vector2F, 
    requests::{
        ClientRequest, 
        ClientResponse, 
        GameplayStateBrief
    }
};

use super::{
    components::{
        GuiIndicator, 
        GuiToggleButton
    }, 
    AppGuiTransition, 
    GuiElement, 
    GuiLayout
};

use std::{
    cell::RefCell, 
    rc::Rc, 
    time::Duration
};

const UPDATE_REQUESTS_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Debug)]
pub struct LobbyGuiLayout {
    pub ready_toggle: GuiToggleButton,
    pub game_starting_indicator: GuiIndicator,
    pub players_list_indicators: Vec<GuiIndicator>,
    pub app_data: Rc<RefCell<AppData>>,
    update_time_accumulator: Duration
}

impl GuiLayout for LobbyGuiLayout {
    fn new(app_data: Rc<RefCell<AppData>>) -> Self {
        log::info!("Entered 'Lobby' gui");
        let app_data_cloned = app_data.clone();
        let (width, height) = {
            let app_data_borrowed = app_data_cloned.borrow();
            (app_data_borrowed.last_width, app_data_borrowed.last_height)
        };

        let mut ready_toggle = build_gui_toggle_button(
            Vector2F::zero(), 
            GuiComponentSize::Big
        );

        let game_starting_indicator = build_gui_indicator(
            Vector2F::zero(), 
            GuiComponentSize::Big
        );

        ready_toggle.set_turned_on(false);

        let mut result = Self {
            app_data,
            game_starting_indicator,
            ready_toggle,
            players_list_indicators: Vec::new(),
            update_time_accumulator: Duration::from_millis(0),
        };
        result.resize_window(width, height);
        result
    }

    fn process_mouse_events(&mut self, position: winit::dpi::PhysicalPosition<f64>, button_state: winit::event::ElementState, button: winit::event::MouseButton) {
        if button_state == ElementState::Released && button == MouseButton::Left {
            let mouse_pos = Vector2F::new(position.x as f32, position.y as f32);
            if self.ready_toggle.is_inside(&mouse_pos) {
                log::info!("Button clicked");
                self.ready_toggle.toggle();
                let should_be_ready = self.ready_toggle.is_on();

                let response = {
                    let app_data = self.app_data.borrow();
                    let cleint_handle = app_data.client_handler.as_ref().unwrap();
                    cleint_handle.make_request(crate::requests::ClientRequest::SetReady { ready: should_be_ready }).unwrap()
                };

                match response {
                    crate::requests::ClientResponse::SetReady { was_set } => {
                        log::info!("Ready was toggled to {}", was_set);
                        // Probably nothing, poll somewhere for start
                    },
                    _ => {
                        log::warn!("Could not toggle ready, response={response:?}");
                        self.ready_toggle.toggle(); // Untoggle 
                    },
                }
            }
        }
    }

    fn resize_window(&mut self, width: f32, height: f32) {
        self.ready_toggle.rect.pos.x = (width - self.ready_toggle.rect.size.x) / 2.0;
        self.ready_toggle.rect.pos.y = (height - self.ready_toggle.rect.size.y) / 2.0;

        const SEPARATOR_GAP: f32 = 32.0;

        self.game_starting_indicator.rect.pos.x = (width - self.game_starting_indicator.rect.size.x) / 2.0;
        self.game_starting_indicator.rect.pos.y = self.ready_toggle.rect.pos.y - self.game_starting_indicator.rect.size.y - SEPARATOR_GAP;
    }

    fn draw(&self, renderer: &mut crate::app::client::gui_client::renderer::Renderer) {
        let (gui_box_1, gui_box_2, gui_box_3) = self.ready_toggle.get_drawable_rects();
        renderer.batch_append_gui_element(GuiElement::Box(gui_box_1));
        renderer.batch_append_gui_element(GuiElement::Box(gui_box_2));
        renderer.batch_append_gui_element(GuiElement::Box(gui_box_3));
        
        let gui_box_4 = self.game_starting_indicator.get_drawable_rects();
        renderer.batch_append_gui_element(GuiElement::Box(gui_box_4));

        self.players_list_indicators.iter().for_each(|indicator| {
            let gui_box = indicator.get_drawable_rects();
            renderer.batch_append_gui_element(GuiElement::Box(gui_box));
        });
    }

    fn update(&mut self, dt: std::time::Duration) {
        self.update_time_accumulator += dt;
        if self.update_time_accumulator > UPDATE_REQUESTS_INTERVAL {
            self.update_time_accumulator -= UPDATE_REQUESTS_INTERVAL;

            let response_gameplay_state = {
                let app_data = self.app_data.borrow();
                let cleint_handle = app_data.client_handler.as_ref().unwrap();
                cleint_handle.make_request(ClientRequest::CheckGameplayState).unwrap()
            };

            if let ClientResponse::CheckGameplayState { state } = response_gameplay_state {
                match state {
                    GameplayStateBrief::Lobby { counting_to_start, last_result: _ } => {
                        let game_is_starting = counting_to_start.is_some();
                        self.game_starting_indicator.set_turned_on(game_is_starting);
                    },
                    GameplayStateBrief::GameRunning => {
                        let mut app_data_borrowed = self.app_data.borrow_mut();
                        app_data_borrowed.app_gui_expected_transition = Some(AppGuiTransition::ToIngame);
                    },
                    GameplayStateBrief::Ending { countdown: _, result: _ } => {
                        log::warn!("Invalid state, client has lobby GUI but server is in ending.")
                    },
                }
            }
            

            // TODO need get all players info, and show how many players are conencted and in lobby

            // TODO need progressbar to indicate countdown till start
        }
    }
}