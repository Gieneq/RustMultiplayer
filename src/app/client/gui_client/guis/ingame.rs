use clap::builder::styling::RgbColor;
use winit::{event::ElementState, keyboard::Key};

use crate::{app::{client::gui_client::{guis::components::templates::{build_gui_progress_bar, GuiComponentSize}, AppData, EntityView}, SEEKING_MAX_TIME}, game::{math::{Rect2F, Vector2F}, world::PlayerRole}, requests::{ClientRequest, ClientResponse, EntityType, MoveDirection}};

use super::{components::{GuiProgressBar, PlayerRoleLayout}, GuiBox, GuiElement, GuiLayout};
use std::{cell::RefCell, rc::Rc, time::Duration};



#[derive(Debug)]
pub struct IngameGuiLayout {
    pub app_data: Rc<RefCell<AppData>>,
    entity_view_list: Vec<EntityView>,
    update_time_accumulator: Duration,
    role_layout: Option<PlayerRoleLayout>,

    is_seeker: bool,
    remaining_time_progress_bar: Option<GuiProgressBar>,
    remaining_tries_count: usize
}
const SCROLL_SENSITIVITY: f32 = 0.1;
const UPDATE_NOT_FREQUENT_INTERVAL: Duration = Duration::from_millis(250);

impl GuiLayout for IngameGuiLayout {
    fn new(app_data: Rc<RefCell<AppData>>) -> Self {
        log::info!("Entered 'Ingame' gui");
        let app_data_cloned = app_data.clone();
        let (width, height) = {
            let app_data_borrowed = app_data_cloned.borrow();
            (app_data_borrowed.last_width, app_data_borrowed.last_height)
        };

        let response = {
            let app_data = app_data_cloned.borrow();
            let cleint_handle = app_data.client_handler.as_ref().unwrap();
            cleint_handle.make_request(ClientRequest::GetRole).unwrap()
        };

        //TODO select seeker/hider layout
        let is_seeker = match response {
            ClientResponse::GetRole { role } => matches!(role, PlayerRole::Seeker { stats: _ }),
            _ => {
                panic!("Bad response to GetRole: {response:?}")
            }
        };

        let mut result = Self {
            app_data,
            entity_view_list: Vec::new(),
            update_time_accumulator: Duration::from_millis(0),
            role_layout: None,
            is_seeker,
            remaining_time_progress_bar: None,
            remaining_tries_count: 0,
        };
        result.resize_window(width, height);
        result
    }

    fn resize_window(&mut self, width: f32, _height: f32) {
        // const SEPARATOR_GAP: f32 = 32.0;
        const PROGR_BAR_MARGINS: f32 = 8.0;

        if let Some(progress_bar) = &mut self.remaining_time_progress_bar {
            progress_bar.rect.size.x = width - 2.0 * PROGR_BAR_MARGINS;
            progress_bar.rect.pos.x = PROGR_BAR_MARGINS;
            progress_bar.rect.pos.y = PROGR_BAR_MARGINS;
        }

    }

    fn process_mouse_wheele(&mut self, delta: winit::event::MouseScrollDelta) {
        match delta {
            winit::event::MouseScrollDelta::LineDelta(_, y) => {
                // y is +-1
                let mut app_data = self.app_data.borrow_mut();
                app_data.world_scale *= (1.0 + SCROLL_SENSITIVITY).powf(y);
            },
            winit::event::MouseScrollDelta::PixelDelta(_physical_position) => { },
        }
    }
    
    /// Must be refactored. Too much option, seeker/hider shoudl has dedicated GUI layout
    fn update(&mut self, dt: std::time::Duration) {
        self.update_time_accumulator += dt;

        if self.update_time_accumulator > UPDATE_NOT_FREQUENT_INTERVAL {
            self.update_time_accumulator -= UPDATE_NOT_FREQUENT_INTERVAL;

            // Game state and similar
        
            let response = {
                let app_data = self.app_data.borrow();
                let cleint_handle = app_data.client_handler.as_ref().unwrap();
                cleint_handle.make_request(ClientRequest::GetRole).unwrap()
            };

            if let ClientResponse::GetRole { role } = response {
                match role {
                    PlayerRole::Hider { stats } => {
                        
                    },
                    PlayerRole::Seeker { stats } => {
                        // Seeker remaining time
                        if self.remaining_time_progress_bar.is_none() {
                            self.remaining_time_progress_bar = Some(build_gui_progress_bar(Vector2F::zero(), GuiComponentSize::Small));
                            let (width, height) = {
                                let app_data = self.app_data.borrow();
                                (app_data.last_width, app_data.last_height)
                            };

                            self.resize_window(width, height);
                        }

                        if let Some(progres_bar) = &mut self.remaining_time_progress_bar {
                            let remaining_time_percentage = 100.0 * stats.remaining_ticks as f32 / SEEKING_MAX_TIME as f32;
                            // println!("remaining_progress={}, {}", remaining_time_percentage, stats.remaining_ticks);
                            progres_bar.set_percantage(remaining_time_percentage);
                        }

                        // Seeker hearts
                        self.remaining_tries_count = stats.remaining_failures;
                    },
                }
            }
        }

        // Entities positions
        let response = {
            let app_data = self.app_data.borrow();
            let cleint_handle = app_data.client_handler.as_ref().unwrap();
            cleint_handle.make_request(ClientRequest::WorldCheck).unwrap()
        };

        if let ClientResponse::WorldCheck { entities } = response {
            // Update visible entities
            self.entity_view_list.clear();

            entities.iter().for_each(|entity| {
                let marker_color = match (self.is_seeker, &entity.entity_type) {
                    (_, EntityType::Npc) => None,
                    (true, EntityType::Hider) => {
                        // I'm seeker, I dont recognise hiders, no mark
                        None
                    },
                    (false, EntityType::Hider) => {
                        // I'm hider, other hiders are my allies, mark them green
                        Some(RgbColor(0, 255, 0))
                    },
                    (true, EntityType::Seeker) => {
                        // I'm seeker, Mark me blue
                        Some(RgbColor(0, 0, 255))
                    },
                    (false, EntityType::Seeker) => {
                        // I'm hider, seeker is my enemy, mark him red
                        Some(RgbColor(255, 0, 0))
                    },
                };

                let entity_view = EntityView {
                    rect: Rect2F {
                        pos: entity.position,
                        size: entity.size,
                    },
                    color: RgbColor(entity.color[0], entity.color[1], entity.color[2]),
                    marker_color
                };
                self.entity_view_list.push(entity_view);
            });

        }
    }

    fn process_key_event(&mut self, event: winit::event::KeyEvent) {
        if event.state == ElementState::Released {
            let app_data = self.app_data.borrow();
            let client_handler = app_data.client_handler.as_ref().unwrap();
            match event.logical_key {
                Key::Named(winit::keyboard::NamedKey::ArrowUp) => {
                    let _ = client_handler.make_request(ClientRequest::Move { dir: MoveDirection::Up} );
                },
                Key::Named(winit::keyboard::NamedKey::ArrowRight) => {
                    let _ = client_handler.make_request(ClientRequest::Move { dir: MoveDirection::Right} );
                },
                Key::Named(winit::keyboard::NamedKey::ArrowDown) => {
                    let _ = client_handler.make_request(ClientRequest::Move { dir: MoveDirection::Down} );
                },
                Key::Named(winit::keyboard::NamedKey::ArrowLeft) => {
                    let _ = client_handler.make_request(ClientRequest::Move { dir: MoveDirection::Left} );
                },
                _ => {}
            }
        } 
    }

    fn draw(&self, renderer: &mut crate::app::client::gui_client::renderer::Renderer) {
        if let Some(progress_bar) = &self.remaining_time_progress_bar {
            let (gui_box_1, gui_box_2, gui_box_3) = progress_bar.get_drawable_rects();
            renderer.batch_append_gui_element(GuiElement::Box(gui_box_1));
            renderer.batch_append_gui_element(GuiElement::Box(gui_box_2));
            renderer.batch_append_gui_element(GuiElement::Box(gui_box_3));
        }

        
        const REMAINING_TRIES_TOP_OFFSET: f32 = 40.0;
        const REMAINING_TRIES_MARGIN: f32 = 8.0;
        const TRIES_SIZE: f32 = 24.0;
        for i in 0..self.remaining_tries_count {
            let gui_box = GuiBox {
                rect: Rect2F::new(
                    REMAINING_TRIES_MARGIN + i as f32 * (TRIES_SIZE + REMAINING_TRIES_MARGIN), 
                    REMAINING_TRIES_TOP_OFFSET, 
                    TRIES_SIZE, 
                    TRIES_SIZE
                ),
                color: RgbColor(209, 6, 57),
            };
            renderer.batch_append_gui_element(GuiElement::Box(gui_box));
        }
        
        self.entity_view_list.iter().for_each(|entity_view| {
            renderer.batch_append_entity_view(*entity_view);
        });
    }
}





