use clap::builder::styling::RgbColor;

use winit::{
    dpi::PhysicalPosition, 
    event::{
        ElementState, 
        MouseButton
    }, 
    keyboard::Key
};

use crate::{
    app::{
        client::gui_client::{
            guis::components::templates::{
                build_gui_progress_bar, 
                GuiComponentSize
            }, 
            AppData, 
            EntityView
        }, 
        SEEKING_MAX_TIME
    }, game::{
        math::{
            Rect2F, 
            Vector2F
        }, 
        world::PlayerRole
    }, 
    requests::{
        ClientRequest, 
        ClientResponse, 
        EntityType, 
        GameplayStateBrief, 
        MoveDirection
    }
};

use super::{
    components::GuiProgressBar, 
    AppGuiTransition, 
    GuiBox, 
    GuiElement, 
    GuiLayout
};

use std::{
    cell::RefCell, 
    ops::Mul, 
    rc::Rc, 
    time::Duration
};



#[derive(Debug)]
pub struct IngameGuiLayout {
    pub app_data: Rc<RefCell<AppData>>,
    entity_view_list: Vec<EntityView>,
    update_time_accumulator: Duration,

    is_seeker: bool,
    remaining_time_progress_bar: Option<GuiProgressBar>,
    remaining_tries_count: usize,
    last_world_mouse_position: Option<Vector2F>,
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
            is_seeker,
            remaining_time_progress_bar: None,
            remaining_tries_count: 0,
            last_world_mouse_position: None,
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

    fn process_mouse_events(&mut self, _position: PhysicalPosition<f64>, button_state: ElementState, button: MouseButton) {
        if let Some(world_mouse_position) = self.last_world_mouse_position{
            if button_state == ElementState::Released && button == MouseButton::Left && self.is_seeker {
                println!("Seeker clicked: {}", world_mouse_position);

                let response = {
                    let app_data = self.app_data.borrow();
                    let cleint_handle = app_data.client_handler.as_ref().unwrap();
                    cleint_handle.make_request(ClientRequest::WorldCheck).unwrap()
                };

                let suspicious_entity_id = if let ClientResponse::WorldCheck { entities } = response {
                    entities.iter().find(|e| {
                        let rect = Rect2F {
                            pos: e.position,
                            size: e.size
                        };
                        rect.contains(&world_mouse_position)
                    })
                    .and_then(|e| Some(e.id))
                } else {
                    None
                };

                if let Some(suspicious_entity_id) = suspicious_entity_id {
                    let _response = {
                        let app_data = self.app_data.borrow();
                        let cleint_handle = app_data.client_handler.as_ref().unwrap();
                        cleint_handle.make_request(ClientRequest::TryUncover { id: suspicious_entity_id }).unwrap()
                    };
                }
            }
        }
    }

    fn mouse_move(&mut self, mouse_position: Vector2F) {
        let (camera_position, world_scale, window_size) = {
            let app_data = self.app_data.borrow();
            (app_data.camera, app_data.world_scale, Vector2F::new(app_data.last_width, app_data.last_height))
        };
        
        let aspect_ratio = window_size.x / window_size.y;
        let scale_x = world_scale / aspect_ratio;
        let scale_y = world_scale;

        let mouse_pos_ndc = Vector2F {
            x: (2.0 * mouse_position.x / window_size.x) - 1.0,
            y: -((2.0 * mouse_position.y / window_size.y) - 1.0),
        };

        let world_mouse_position = Vector2F {
            x: mouse_pos_ndc.x / scale_x + camera_position.x,
            y: mouse_pos_ndc.y / scale_y + camera_position.y
        };
        
        self.last_world_mouse_position = Some(world_mouse_position);
        // println!("world_mouse_position={world_mouse_position}, mouse_pos_ndc={mouse_pos_ndc}, mouse_position={mouse_position}, camera_position={camera_position}, world_scale={world_scale}");
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
                cleint_handle.make_request(ClientRequest::CheckGameplayState).unwrap()
            };

            if let ClientResponse::CheckGameplayState { state } = response {
                match state {
                    GameplayStateBrief::Lobby { counting_to_start: _, last_result: _ } => {
                        log::warn!("Invalid state, client has lobby GUI but server is in ending.")
                    },
                    GameplayStateBrief::GameRunning => { },
                    GameplayStateBrief::Ending { countdown: _, result: _ } => {
                        let mut app_data_borrowed = self.app_data.borrow_mut();
                        app_data_borrowed.app_gui_expected_transition = Some(AppGuiTransition::ToEnding);
                    },
                }
            }

        
            let response = {
                let app_data = self.app_data.borrow();
                let cleint_handle = app_data.client_handler.as_ref().unwrap();
                cleint_handle.make_request(ClientRequest::GetRole).unwrap()
            };

            if let ClientResponse::GetRole { role } = response {
                match role {
                    PlayerRole::Hider { stats: _ } => {
                        
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
            // Extract observed entity position 
            let extract_observed_entity_position = || {
                let response = {
                    let app_data = self.app_data.borrow();
                    let cleint_handle = app_data.client_handler.as_ref().unwrap();
                    cleint_handle.make_request(ClientRequest::GetEntityId).unwrap()
                };

                if let ClientResponse::GetEntityId { id } = response {
                    if let Some(entity_id) = id {
                        let found_entity = entities.iter()
                            .find(|e| e.id == entity_id);
                        if let Some(found_entity) = found_entity {
                            return Some(found_entity.position);
                        }
                    }
                }
                None
            };

            // Update camera
            if let Some(observed_entity_pos) = extract_observed_entity_position() {
                const SMOOTHING_ALPHA: f32 = 0.09;
                let mut app_data = self.app_data.borrow_mut();
                let delta_pos = (observed_entity_pos - app_data.camera).mul(SMOOTHING_ALPHA);
                app_data.camera += delta_pos
            }

            // Update visible entities
            self.entity_view_list.clear();

            entities.iter().for_each(|entity| {
                let rect = Rect2F {
                    pos: entity.position,
                    size: entity.size,
                };

                let marker_color = match (self.is_seeker, &entity.entity_type) {
                    (_, EntityType::Npc) => None,
                    (true, EntityType::Hider { covered: true }) => {
                        // I'm seeker, I dont recognise covered hiders
                        None
                    },
                    (true, EntityType::Hider { covered: false }) => {
                        // I'm seeker, I dont recognise covered hiders
                        Some(RgbColor(30, 255, 0))
                    },
                    (false, EntityType::Hider { covered: _}) => {
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

                // Highlighting
                let highlighted = if self.is_seeker && matches!(entity.entity_type, EntityType::Seeker) {
                    false
                } else if !self.is_seeker {
                    false
                } else if let Some(world_mouse_position) = self.last_world_mouse_position {
                    rect.contains(&world_mouse_position)
                } else {
                    false
                };

                let entity_view = EntityView {
                    rect,
                    color: RgbColor(entity.color[0], entity.color[1], entity.color[2]),
                    marker_color,
                    highlighted
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

        const REMAINING_TRIES_TOP_OFFSET: f32 = 48.0;
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





