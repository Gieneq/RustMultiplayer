pub mod disconnected;
pub mod lobby;
pub mod ingame;
pub mod ending;

use std::{cell::RefCell, rc::Rc};

use clap::builder::styling::RgbColor;
use winit::{dpi::PhysicalPosition, event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta}};

use crate::game::math::Rect2F;

use super::{renderer::Renderer, AppData};
use self::{disconnected::DisconnectedGuiLayout, ending::EndingGuiLayout, ingame::IngameGuiLayout, lobby::LobbyGuiLayout};

#[derive(Debug)]
pub struct GuiBox {
    pub rect: Rect2F,
    pub color: RgbColor
}

#[derive(Debug)]
pub enum GuiElement {
    Box(GuiBox)
}

pub mod components {
    use clap::builder::styling::RgbColor;

    use crate::game::math::{Rect2F, Vector2F};

    use super::GuiBox;

    #[derive(Debug)]
    pub struct GuiPlainButton {
        pub rect: Rect2F,
        pub active: bool,
        pub color_middle: RgbColor,
        pub color_outer: RgbColor,
    }

    impl GuiPlainButton {
        const BORDER_SIZE: f32 = 4.0;
        pub fn new(rect: Rect2F, color_middle: RgbColor, color_outer: RgbColor) -> Self {
            Self { rect, active: true, color_middle, color_outer }
        }

        pub fn get_drawable_rects(&self) -> (GuiBox, GuiBox) {
            (
                GuiBox {
                    rect: self.rect,
                    color: self.color_outer
                },
                GuiBox {
                    rect: Rect2F::new(
                        self.rect.pos.x + Self::BORDER_SIZE, 
                        self.rect.pos.y + Self::BORDER_SIZE, 
                        self.rect.size.x - 2.0 * Self::BORDER_SIZE,
                        self.rect.size.y - 2.0 * Self::BORDER_SIZE
                    ),
                    color: self.color_middle
                }
            )
        }

        pub fn is_inside(&self, point: &Vector2F) -> bool {
            self.rect.contains(point)
        }
    }
}

pub trait GuiLayout {
    fn new(app_data: Rc<RefCell<AppData>>) -> Self;

    fn resize_window(&mut self, width: f32, height: f32) { }

    fn draw(&self, renderer: &mut Renderer) { }
    
    fn process_key_event(&mut self, event: KeyEvent) { }

    fn process_mouse_wheele(&mut self, delta: MouseScrollDelta) { }

    fn process_mouse_events(&mut self, position: PhysicalPosition<f64>, button_state: ElementState, button: MouseButton) { }

    fn update(&mut self, dt: std::time::Duration) { }
}

#[derive(Debug)]
pub enum AppGuiTransition {
    ToLobby,
    ToDisconnected,
    ToIngame,
    ToEnding,
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
        gui: IngameGuiLayout,
    },
    Ending {
        gui: EndingGuiLayout,
    },
}

impl GuiLayout for AppGui {
    fn new(app_data: Rc<RefCell<AppData>>) -> Self {
        Self::Disconnected { gui: DisconnectedGuiLayout::new(app_data) }
    }

    fn resize_window(&mut self, width: f32, height: f32) {
        match self {
            AppGui::Disconnected { gui } => gui.resize_window(width, height),
            AppGui::Lobby { gui }  => gui.resize_window(width, height),
            AppGui::Ingame { gui }  => gui.resize_window(width, height),
            AppGui::Ending { gui }  => gui.resize_window(width, height),
        }
    }

    fn process_mouse_events(&mut self, position: PhysicalPosition<f64>, button_state: ElementState, button: MouseButton) {
        println!("process_mouse_events: {position:?}, {button_state:?}, {button:?}");

        match self {
            AppGui::Disconnected { gui } => gui.process_mouse_events(position, button_state, button),
            AppGui::Lobby { gui }  => gui.process_mouse_events(position, button_state, button),
            AppGui::Ingame { gui }  => gui.process_mouse_events(position, button_state, button),
            AppGui::Ending { gui }  => gui.process_mouse_events(position, button_state, button),
        }
    }

    fn draw(&self, renderer: &mut Renderer) {
        match self {
            AppGui::Disconnected { gui } => gui.draw(renderer),
            AppGui::Lobby { gui }  => gui.draw(renderer),
            AppGui::Ingame { gui }  => gui.draw(renderer),
            AppGui::Ending { gui }  => gui.draw(renderer),
        }
    }
}

impl AppGui {
    pub fn get_app_data(&self) -> Rc<RefCell<AppData>> {
        match self {
            AppGui::Disconnected { gui } => gui.app_data.clone(),
            AppGui::Lobby { gui } => gui.app_data.clone(),
            AppGui::Ingame { gui } => gui.app_data.clone(),
            AppGui::Ending { gui } => gui.app_data.clone(),
        }
    }

    pub fn transition(&mut self, transition_to: AppGuiTransition) {
        *self = match transition_to {
            AppGuiTransition::ToLobby => AppGui::Lobby { gui: LobbyGuiLayout::new(self.get_app_data()) },
            AppGuiTransition::ToDisconnected => AppGui::Disconnected { gui: DisconnectedGuiLayout::new(self.get_app_data()) },
            AppGuiTransition::ToIngame => AppGui::Ingame { gui: IngameGuiLayout::new(self.get_app_data()) },
            AppGuiTransition::ToEnding => AppGui::Ending {  gui: EndingGuiLayout::new(self.get_app_data()) },
        };
    }
}