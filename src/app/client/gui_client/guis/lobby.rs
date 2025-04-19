use crate::app::client::gui_client::AppData;

use super::GuiLayout;
use std::{cell::RefCell, rc::Rc};

#[derive(Debug)]
pub struct LobbyGuiLayout {
    pub app_data: Rc<RefCell<AppData>>
}

impl GuiLayout for LobbyGuiLayout {
    fn new(app_data: Rc<RefCell<AppData>>) -> Self {
        log::info!("Entered 'Lobby' gui");

        Self {
            app_data
        }
    }
}