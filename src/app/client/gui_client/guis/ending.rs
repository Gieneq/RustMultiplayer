use std::{cell::RefCell, rc::Rc};
use crate::app::client::gui_client::AppData;

use super::GuiLayout;

#[derive(Debug)]
pub struct EndingGuiLayout {
    pub app_data: Rc<RefCell<AppData>>
}

impl GuiLayout for EndingGuiLayout {
    fn new(app_data: Rc<RefCell<AppData>>) -> Self {
        Self { app_data }
    }
}