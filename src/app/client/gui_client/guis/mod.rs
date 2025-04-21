pub mod disconnected;
pub mod lobby;
pub mod ingame;
pub mod ending;

use std::{cell::RefCell, rc::Rc};

use clap::builder::styling::RgbColor;
use winit::{dpi::PhysicalPosition, event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta}};

use crate::game::math::{Rect2F, Vector2F};

use super::{renderer::Renderer, AppData};
use self::{disconnected::DisconnectedGuiLayout, ending::EndingGuiLayout, ingame::IngameGuiLayout, lobby::LobbyGuiLayout};

#[derive(Debug, Clone, Copy)]
pub struct GuiBox {
    pub rect: Rect2F,
    pub color: RgbColor
}

#[derive(Debug)]
pub enum GuiElement {
    Box(GuiBox)
}

const BORDER_SIZE: f32 = 8.0;

pub mod components {
    use clap::builder::styling::RgbColor;

    use crate::game::math::{Rect2F, Vector2F};

    use super::GuiBox;

    #[derive(Debug)]
    pub struct GuiPlainButton {
        pub rect: Rect2F,
        pub color_middle: RgbColor,
        pub color_outer: RgbColor,
    }

    impl GuiPlainButton {
        pub fn new(rect: Rect2F, color_middle: RgbColor, color_outer: RgbColor) -> Self {
            Self { rect, color_middle, color_outer }
        }

        pub fn get_drawable_rects(&self) -> (GuiBox, GuiBox) {
            (
                GuiBox {
                    rect: self.rect,
                    color: self.color_outer
                },
                GuiBox {
                    rect: Rect2F::new(
                        self.rect.pos.x + super::BORDER_SIZE, 
                        self.rect.pos.y + super::BORDER_SIZE, 
                        self.rect.size.x - 2.0 * super::BORDER_SIZE,
                        self.rect.size.y - 2.0 * super::BORDER_SIZE
                    ),
                    color: self.color_middle
                }
            )
        }

        pub fn is_inside(&self, point: &Vector2F) -> bool {
            self.rect.contains(point)
        }
    }
    
    #[derive(Debug)]
    pub struct GuiToggleButton {
        pub rect: Rect2F,
        pub turned_on: bool,
        pub color_middle_on: RgbColor,
        pub color_middle_off: RgbColor,
        pub color_outer: RgbColor,
    }

    impl GuiToggleButton {
        pub fn new(rect: Rect2F, color_middle_on: RgbColor, color_middle_off: RgbColor, color_outer: RgbColor) -> Self {
            Self { rect, turned_on: false, color_middle_on, color_middle_off, color_outer }
        }
        
        pub fn get_drawable_rects(&self) -> (GuiBox, GuiBox, GuiBox) {
            let inner_rest = Rect2F::new(
                self.rect.pos.x + super::BORDER_SIZE, 
                self.rect.pos.y + super::BORDER_SIZE, 
                self.rect.size.x - 2.0 * super::BORDER_SIZE,
                self.rect.size.y - 2.0 * super::BORDER_SIZE
            );

            let overlay_rect = Rect2F::new(
                inner_rest.pos.x + if self.turned_on {
                    inner_rest.size.x / 2.0
                } else {
                    0.0
                }, 
                inner_rest.pos.y, 
                inner_rest.size.x / 2.0,
                inner_rest.size.y
            );

            (
                GuiBox {
                    rect: self.rect,
                    color: self.color_outer
                },
                GuiBox {
                    rect: inner_rest,
                    color: if self.turned_on {
                        self.color_middle_on
                    } else {
                        self.color_middle_off
                    }
                },
                GuiBox {
                    rect: overlay_rect,
                    color: self.color_outer
                }
            )
        }

        pub fn set_turned_on(&mut self, turned_on: bool) {
            self.turned_on = turned_on;
        }

        pub fn toggle(&mut self) {
            self.turned_on = !self.turned_on;
        }

        pub fn is_on(&self) -> bool {
            self.turned_on
        }

        pub fn is_inside(&self, point: &Vector2F) -> bool {
            self.rect.contains(point)
        }
    }

    #[derive(Debug)]
    pub struct GuiIndicator {
        pub rect: Rect2F,
        pub turned_on: bool,
        pub color_middle_on: RgbColor,
        pub color_middle_off: RgbColor,
    }

    impl GuiIndicator {
        pub fn new(rect: Rect2F, color_middle_on: RgbColor, color_middle_off: RgbColor) -> Self {
            Self { rect, turned_on: false, color_middle_on, color_middle_off }
        }

        pub fn get_drawable_rects(&self) -> GuiBox {
            GuiBox {
                rect: self.rect,
                color: if self.turned_on {
                    self.color_middle_on
                } else {
                    self.color_middle_off
                }
            }
        }

        pub fn set_turned_on(&mut self, turned_on: bool) {
            self.turned_on = turned_on;
        }

        pub fn toggle(&mut self) {
            self.turned_on = !self.turned_on;
        }

        pub fn is_on(&self) -> bool {
            self.turned_on
        }
    }

    #[derive(Debug)]
    pub enum PlayerRoleLayout {
        Seeker(SeekerLayout),
        Hider(HiderLayout)
    }

    #[derive(Debug)]
    pub struct SeekerLayout {

    }

    #[derive(Debug)]
    pub struct HiderLayout {

    }

    #[derive(Debug)]
    pub struct GuiProgressBar {
        pub rect: Rect2F,
        pub color_middle: RgbColor,
        pub color_bg: RgbColor,
        pub color_frame: RgbColor,
        percentage: f32,
    }

    impl GuiProgressBar {
        pub fn new(rect: Rect2F, color_middle: RgbColor, color_bg: RgbColor, color_frame: RgbColor) -> Self {
            Self { rect, color_middle, color_bg, color_frame, percentage: 0.0 }
        }

        pub fn set_percantage(&mut self, percentage: f32) {
            self.percentage = percentage.clamp(0.0, 100.0);
        }

        pub fn get_drawable_rects(&self) -> (GuiBox, GuiBox, GuiBox) {
            let inner_rest = Rect2F::new(
                self.rect.pos.x + super::BORDER_SIZE, 
                self.rect.pos.y + super::BORDER_SIZE, 
                self.rect.size.x - 2.0 * super::BORDER_SIZE,
                self.rect.size.y - 2.0 * super::BORDER_SIZE
            );

            let progress_rect = Rect2F::new(
                inner_rest.pos.x, 
                inner_rest.pos.y, 
                inner_rest.size.x * self.percentage / 100.0,
                inner_rest.size.y
            );

            ( 
                GuiBox {
                    rect: self.rect,
                    color: self.color_frame
                },
                GuiBox {
                    rect: inner_rest,
                    color: self.color_bg
                },
                GuiBox {
                    rect: progress_rect,
                    color: self.color_middle
                }
            )
        }
    }

    pub mod templates {
        use clap::builder::styling::RgbColor;

        use crate::game::math::{Rect2F, Vector2F};

        use super::{GuiIndicator, GuiPlainButton, GuiProgressBar, GuiToggleButton};

        pub enum GuiComponentSize {
            Small,
            Medium,
            Big
        }

        pub fn build_gui_plain_button(pos: Vector2F, component_size: GuiComponentSize) -> GuiPlainButton {
            let size = match component_size {
                GuiComponentSize::Small => Vector2F { x: 50.0, y: 25.0 },
                GuiComponentSize::Medium => Vector2F { x: 110.0, y: 60.0 },
                GuiComponentSize::Big => Vector2F { x: 280.0, y: 110.0 },
            };
            GuiPlainButton::new(
                Rect2F { pos, size }, 
                RgbColor(0, 186, 22),
                RgbColor(1, 77, 30)
            )
        }

        pub fn build_gui_toggle_button(pos: Vector2F, component_size: GuiComponentSize) -> GuiToggleButton {
            let size = match component_size {
                GuiComponentSize::Small => Vector2F { x: 50.0, y: 25.0 },
                GuiComponentSize::Medium => Vector2F { x: 110.0, y: 60.0 },
                GuiComponentSize::Big => Vector2F { x: 280.0, y: 110.0 },
            };
            GuiToggleButton::new(
                Rect2F { pos, size }, 
                RgbColor(2, 191, 27), 
                RgbColor(105, 0, 0), 
                RgbColor(10, 10, 10)
            )
        }
        
        pub fn build_gui_indicator(pos: Vector2F, component_size: GuiComponentSize) -> GuiIndicator {
            let size = match component_size {
                GuiComponentSize::Small => Vector2F { x: 16.0, y: 16.0 },
                GuiComponentSize::Medium => Vector2F { x: 32.0, y: 32.0 },
                GuiComponentSize::Big => Vector2F { x: 64.0, y: 64.0 },
            };
            GuiIndicator::new(
                Rect2F { pos, size }, 
                RgbColor(0, 186, 22), 
                RgbColor(45, 61, 47), 
            )
        }

        pub fn build_gui_progress_bar(pos: Vector2F, component_size: GuiComponentSize) -> GuiProgressBar {
            let size = match component_size {
                GuiComponentSize::Small => Vector2F { x: 100.0, y: 32.0 },
                GuiComponentSize::Medium => Vector2F { x: 200.0, y: 48.0 },
                GuiComponentSize::Big => Vector2F { x: 400.0, y: 64.0 },
            };
            GuiProgressBar::new(
                Rect2F { pos, size }, 
                RgbColor(130, 217, 214), 
                RgbColor(38, 38, 38), 
                RgbColor(92, 92, 92)
            )
        }
    }
}

pub trait GuiLayout {
    fn new(app_data: Rc<RefCell<AppData>>) -> Self;

    fn resize_window(&mut self, _width: f32, _height: f32) { }

    fn draw(&self, _renderer: &mut Renderer) { }
    
    fn process_key_event(&mut self, _event: KeyEvent) { }

    fn process_mouse_wheele(&mut self, _delta: MouseScrollDelta) { }

    fn process_mouse_events(&mut self, _position: PhysicalPosition<f64>, _button_state: ElementState, _button: MouseButton) { }

    fn mouse_move(&mut self, _mouse_position: Vector2F) { }

    fn update(&mut self, _dt: std::time::Duration) { }
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

    fn update(&mut self, dt: std::time::Duration) {
        match self {
            AppGui::Disconnected { gui } => gui.update(dt),
            AppGui::Lobby { gui }  => gui.update(dt),
            AppGui::Ingame { gui }  => gui.update(dt),
            AppGui::Ending { gui }  => gui.update(dt),
        }
    }

    fn process_key_event(&mut self, event: KeyEvent) {
        match self {
            AppGui::Disconnected { gui } => gui.process_key_event(event),
            AppGui::Lobby { gui }  => gui.process_key_event(event),
            AppGui::Ingame { gui }  => gui.process_key_event(event),
            AppGui::Ending { gui }  => gui.process_key_event(event),
        }
    }

    fn process_mouse_wheele(&mut self, delta: MouseScrollDelta) {
        match self {
            AppGui::Disconnected { gui } => gui.process_mouse_wheele(delta),
            AppGui::Lobby { gui }  => gui.process_mouse_wheele(delta),
            AppGui::Ingame { gui }  => gui.process_mouse_wheele(delta),
            AppGui::Ending { gui }  => gui.process_mouse_wheele(delta),
        }
    }
    
    fn mouse_move(&mut self, mouse_position: Vector2F) {
        match self {
            AppGui::Disconnected { gui } => gui.mouse_move(mouse_position),
            AppGui::Lobby { gui }  => gui.mouse_move(mouse_position),
            AppGui::Ingame { gui }  => gui.mouse_move(mouse_position),
            AppGui::Ending { gui }  => gui.mouse_move(mouse_position),
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