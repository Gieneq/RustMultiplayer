use std::{cell::RefCell, rc::Rc, time::Duration};
use crate::{app::client::gui_client::AppData, requests::{ClientRequest, ClientResponse, GameplayStateBrief}};

use super::{AppGuiTransition, GuiLayout};

const UPDATE_REQUESTS_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Debug)]
pub struct EndingGuiLayout {
    pub app_data: Rc<RefCell<AppData>>,
    update_time_accumulator: Duration,
}

impl GuiLayout for EndingGuiLayout {
    fn new(app_data: Rc<RefCell<AppData>>) -> Self {
        log::info!("Entered 'Ending' gui");
        Self { 
            app_data,
            update_time_accumulator: Duration::from_millis(0),
        }
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
                    GameplayStateBrief::Lobby { counting_to_start: _, last_result: _ } => {
                        let mut app_data_borrowed = self.app_data.borrow_mut();
                        app_data_borrowed.app_gui_expected_transition = Some(AppGuiTransition::ToLobby);
                    },
                    GameplayStateBrief::GameRunning => {
                        log::warn!("Invalid state, client has lobby GUI but server is in ending.")
                    },
                    GameplayStateBrief::Ending { countdown: _, result: _ } => { },
                }
            }
        }
    }
}