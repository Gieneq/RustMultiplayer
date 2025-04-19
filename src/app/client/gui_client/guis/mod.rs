pub mod disconnected;
pub mod lobby;
pub mod ingame;
pub mod ending;

use winit::event::{KeyEvent, MouseScrollDelta};

use super::{gui_renderer::GuiRenderer, AppIngameStageData};
use self::{disconnected::DisconnectedGuiLayout, ending::EndingGuiLayout, ingame::IngameGuiLayout, lobby::LobbyGuiLayout};

pub trait GuiLayout {
    fn new() -> Self;

    fn draw(&self, gui_renderer: &GuiRenderer) { }
    
    fn process_key_event(&mut self, event: KeyEvent) { }

    fn process_mouse_wheele(&mut self, delta: MouseScrollDelta) { }

    fn update(&mut self, dt: std::time::Duration) { }
}


#[derive(Debug)]
pub enum AppGui {
    Disconnected {
        gui: DisconnectedGuiLayout,
    },
    Lobby {
        gui: LobbyGuiLayout,
    },
    Ingame {
        data: AppIngameStageData,
        gui: IngameGuiLayout,
    },
    Ending {
        gui: EndingGuiLayout,
    },
}

impl GuiLayout for AppGui {
    fn new() -> Self {
        Self::Disconnected { gui: DisconnectedGuiLayout::new() }
    }
}

impl AppGui {
    pub fn enter_lobby(&mut self) {
        log::info!("Entering 'Lobby' gui");
        *self = AppGui::Lobby { gui: LobbyGuiLayout::new() };
    }
}