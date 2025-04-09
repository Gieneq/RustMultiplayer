use rust_multiplayer::{
    client_requests::{ClientRequest, ClientResponse, MoveDirection}, 
    game::common::Vector2F, 
    rendering::{
        renderer::State, AppData, EntityView
    }, TEST_SERVER_ADRESS
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use wgpu::naga::proc::NameKey;

use std::{sync::{Arc, Mutex}, time::Duration};

use winit::{
    application::ApplicationHandler, event::{ElementState, WindowEvent}, event_loop::{
        ActiveEventLoop, 
        ControlFlow, 
        EventLoop
    }, keyboard::Key, window::{
        Window, 
        WindowId
    }
};

#[derive(Default)]
struct App {
    state: Option<State>,
    data: Arc<Mutex<AppData>>,
    client_handler: Option<GuiClientHandle>
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Create window object
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );

        let state = pollster::block_on(State::new(window.clone()));
        self.state = Some(state);

        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let state = self.state.as_mut().unwrap();
        let app_data = self.data.clone();
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();

                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async move {
                    self.client_handler.take().unwrap().wait_until_finished().await.unwrap();
                });
            }
            WindowEvent::RedrawRequested => {
                if let Ok(app_data_guard) = self.data.lock() {
                    state.render(&app_data_guard)
                };

                // Emits a new redraw requested event.
                state.get_window().request_redraw();
            }
            WindowEvent::Resized(size) => {
                // Reconfigures the size of the surface. We do not re-render
                // here as this event is always followed up by redraw request.
                state.resize(size);
            },
            WindowEvent::MouseWheel { 
                device_id: _, 
                delta, 
                phase: _ 
            } => {
                let scroll_sentivity: f32 = 0.1;
                match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => {
                        // y is +-1
                        if let Ok(mut app_data_guard) = self.data.lock() {
                            app_data_guard.scale *= (1.0 + scroll_sentivity).powf(y);
                        }
                    },
                    winit::event::MouseScrollDelta::PixelDelta(_physical_position) => todo!(),
                }
            },
            WindowEvent::KeyboardInput { device_id: _, event, is_synthetic: _ } => {
                if event.state == ElementState::Released {
                    match event.logical_key {
                        Key::Named(winit::keyboard::NamedKey::ArrowUp) => {
                            self.client_handler.as_ref().unwrap().move_headless(MoveDirection::Up);
                        },
                        Key::Named(winit::keyboard::NamedKey::ArrowRight) => {
                            self.client_handler.as_ref().unwrap().move_headless(MoveDirection::Right);
                        },
                        Key::Named(winit::keyboard::NamedKey::ArrowDown) => {
                            self.client_handler.as_ref().unwrap().move_headless(MoveDirection::Down);
                        },
                        Key::Named(winit::keyboard::NamedKey::ArrowLeft) => {
                            self.client_handler.as_ref().unwrap().move_headless(MoveDirection::Left);
                        },
                        _ => {}
                    }
                }    
            }
            _ => (),
        }
    }
}

struct GuiClient {
    socket: tokio::net::TcpStream
}

struct GuiClientHandle {
    task_handle: tokio::task::JoinHandle<()>,
    app_data: Arc<Mutex<AppData>>,
    contol_signals_tx: std::sync::mpsc::Sender<MoveDirection>
}

async fn client_do_request_await_response(
    req: &str,
    buf_reader: &mut tokio::io::BufReader<tokio::net::tcp::ReadHalf<'_>>,
    write: &mut tokio::net::tcp::WriteHalf<'_>,
) -> String {
    let mut buf_string = String::new();

    write.write_all(req.as_bytes()).await.unwrap();
    write.write_all(b"\n").await.unwrap();
    write.flush().await.unwrap();

    buf_reader.read_line(&mut buf_string).await.unwrap();
    buf_string.trim().to_string()
}

impl GuiClient {
    async fn connect<A: tokio::net::ToSocketAddrs + std::fmt::Debug>(addr: A) -> GuiClient {
        log::info!("Client attempts to connect to server {addr:?}...");

        let socket = tokio::net::TcpStream::connect(addr).await.unwrap();
        let client_address = socket.local_addr().unwrap();
        log::info!("Client {client_address} connected!");
        GuiClient {
            socket
        }
    }

    async fn run(mut self, app_data: Arc<Mutex<AppData>>) -> GuiClientHandle {
        let app_data_cloned = app_data.clone();
        let (contol_signals_tx, contol_signals_rx) = std::sync::mpsc::channel();

        let task_handle = tokio::task::spawn(async move {
            let (read_half, mut write_half) = self.socket.split();
            let mut buf_reader = tokio::io::BufReader::new(read_half);

            // store player id
            let player_id = {
                let response = client_do_request_await_response(
                    "{\"type\":\"GetId\"}",
                    &mut buf_reader,
                    &mut write_half
                ).await;

                if let Ok(ClientResponse::GetId { id }) = serde_json::from_str(&response) {
                    id
                } else {
                    panic!("PlayerGetID parse failed")
                }
            };

            loop {
                let response = client_do_request_await_response(
                    "{\"type\":\"WorldCheck\"}",
                    &mut buf_reader,
                    &mut write_half
                ).await;
                log::trace!("Client got response '{response}'.");

                if let ClientResponse::WorldCheck { entities } = serde_json::from_str(&response).unwrap() {
                    // Update shared data
                    if let Ok(mut app_data_guard) = app_data.lock() {
                        app_data_guard.entities.clear();
                        for entiy in entities {
                            if entiy.id == player_id {
                                app_data_guard.camera_position = entiy.position;
                            }

                            let color = [
                                entiy.color[0] as f32 / 255.0,
                                entiy.color[1] as f32 / 255.0,
                                entiy.color[2] as f32 / 255.0
                            ];

                            app_data_guard.entities.push(EntityView { 
                                position: entiy.position, 
                                size: entiy.size, 
                                color
                            });
                            
                        }
                    }
                }

                // Poll for control signals
                if let Ok(move_dir) = contol_signals_rx.try_recv() {
                    let request = serde_json::to_string(&ClientRequest::Move{dir: move_dir}).unwrap();
                    let response = client_do_request_await_response(
                        &request,
                        &mut buf_reader,
                        &mut write_half
                    ).await;
                    log::debug!("Client got response '{response}'.");
                }
                

                tokio::time::sleep(Duration::from_millis(32)).await;
            }

        });

        GuiClientHandle {
            task_handle,
            app_data: app_data_cloned,
            contol_signals_tx
        }
    }
}

impl GuiClientHandle {
    async fn wait_until_finished(self) -> Result<(), tokio::task::JoinError> {
        self.task_handle.await
    }

    fn move_headless(&self, direction: MoveDirection) {
        self.contol_signals_tx.send(direction).unwrap();
    }
}

#[tokio::main]
async fn main() {
    // wgpu uses `log` for all of our logging, so we initialize a logger with the `env_logger` crate.
    
    env_logger::builder()
        .filter_level(log::LevelFilter::Warn)
        .format_timestamp_millis()
        .format_file(false)
        .format_line_number(true)
        .init();


    let event_loop = EventLoop::new().unwrap();

    // When the current loop iteration finishes, immediately begin a new
    // iteration regardless of whether or not new events are available to
    // process. Preferred for applications that want to render as fast as
    // possible, like games.
    event_loop.set_control_flow(ControlFlow::Poll);

    // When the current loop iteration finishes, suspend the thread until
    // another event arrives. Helps keeping CPU utilization low if nothing
    // is happening, which is preferred if the application might be idling in
    // the background.
    // event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App {
        data: Arc::new(Mutex::new(AppData {
            scale: 0.5,
            ..Default::default()
        })),
        ..Default::default()
    };
    
    let client_handler = GuiClient::connect(TEST_SERVER_ADRESS).await
        .run(
            app.data.clone()
    ).await;
    app.client_handler = Some(client_handler);

    event_loop.run_app(&mut app).unwrap();

    // client_handler.clone().wait_until_finished().await.unwrap();
}