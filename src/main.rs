use clap::{Parser, Subcommand, Args};
use rust_multiplayer::DEFAULT_SERVER_ADRESS;

/// # Global Arguments
#[derive(Debug, Parser)]
#[command(version, about = "Multiplayer game", long_about = None)]
struct Cli {
    #[command(subcommand)]
    mode: Mode,
}

#[derive(Debug, Subcommand)]
enum Mode {
    /// Run server
    Server(ServerArgs),

    /// Request server
    Request(RequestArgs),

    /// Run player cleint app
    Player(PlayerClientArgs)
}

#[derive(Debug, Args)]
struct ServerArgs {
    /// Server address
    #[arg(short = 'a', long = "address", value_name = "SERVER_ADDRESS", default_value_t = String::from(DEFAULT_SERVER_ADRESS))]
    address: String,
}

#[derive(Debug, Args)]
struct RequestArgs {
    /// Server address
    #[arg(short = 'a', long = "address", value_name = "SERVER_ADDRESS", default_value_t = String::from(DEFAULT_SERVER_ADRESS))]
    address: String,
}

#[derive(Debug, Args)]
struct PlayerClientArgs {
    /// Server address
    #[arg(short = 'a', long = "address", value_name = "SERVER_ADDRESS", default_value_t = String::from(DEFAULT_SERVER_ADRESS))]
    address: String,
    
    /// Player name
    #[arg(short = 'n', long = "name", value_name = "PLAYER_NAME", required = true)]
    player_name: String,
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"))
        .format_timestamp_millis()
        .format_file(false)
        .format_line_number(true)
        .init();

    let cli_args = Cli::parse();
    log::info!("Got args: '{:?}'.", cli_args);
    
    match cli_args.mode {
        Mode::Server(server_args) => {
            cli_server::run(&server_args.address);
        },
        Mode::Request(request_args) => {
            cli_request::run(&request_args.address);
        },
        Mode::Player(player_client_args) => {
            cli_player_client::run(&player_client_args.address, &player_client_args.player_name);
        },
    }
}

mod cli_server {
    use rust_multiplayer::{
        game::math::Vector2F, 
        app::server::MultiplayerServer
    };

    pub fn run<A: tokio::net::ToSocketAddrs>(addr: A) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let server = MultiplayerServer::bind(addr).await.unwrap();
            log::info!("MP-server, address:{:?}",  server.get_local_address().unwrap());
            
            let server_handler = server.run().await.unwrap();
        
            {
                let mut world = server_handler.world.lock().unwrap();
                world.create_entity_npc("Tuna", Vector2F::new(5.0, 10.0), Vector2F::new(4.8, 4.8));
                world.create_entity_npc("Starlette", Vector2F::new(-5.0, 0.0), Vector2F::new(4.8, 4.8));
                world.create_entity_npc("Bucket", Vector2F::new(5.0, -5.0), Vector2F::new(4.8, 4.8));
                world.create_entity_npc("Sugar", Vector2F::new(5.0, 0.0), Vector2F::new(4.8, 4.8));
                world.create_entity_npc("Tapioka", Vector2F::new(10.0, 5.0), Vector2F::new(4.8, 4.8));

                for ix in -9..9 {
                    if (-2..3).contains(&ix) {
                        continue;
                    }
                    for iy in -5..5 {
                        if (-2..3).contains(&iy) {
                            continue;
                        }
                        let x = (ix * 5) as f32;
                        let y = (iy * 5) as f32;
                        world.create_entity_npc("Bot", Vector2F::new(x, y), Vector2F::new(4.8, 4.8));
                    }
                }
            }

            let (ctrlc_sender, ctrlc_receiver) = tokio::sync::oneshot::channel();
            let mut ctrlc_sender = Some(ctrlc_sender);

            ctrlc::set_handler(move || {
                log::info!("Captured ctrl-C, shutting down the server...");
                let sndr = ctrlc_sender.take().unwrap();
                sndr.send(()).unwrap();
            }).expect("Error setting Ctrl-C handler");

            ctrlc_receiver.await.unwrap();
            server_handler.shutdown().await.unwrap();
        })
    }
}

mod cli_request {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

    use tokio::net::TcpStream;
    use std::net::SocketAddr;


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

    pub fn run<A: tokio::net::ToSocketAddrs + std::fmt::Display>(addr: A) {
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async move {
            log::info!("Client attempts to connect to server {addr}...");

            let mut socket = TcpStream::connect(addr).await.unwrap();
            let client_address: SocketAddr = socket.local_addr().unwrap();
            log::info!("Client {client_address} connected!");

            let (read_half, mut write_half) = socket.split();
            let mut buf_reader = tokio::io::BufReader::new(read_half);

            let requests = [
                String::from("{\"type\":\"Healthcheck\"}"),
                String::from("{\"type\":\"GetId\"}"),
                String::from("{\"type\":\"WorldCheck\"}"),
            ];

            for request in requests {
                let response = client_do_request_await_response(
                    &request,
                    &mut buf_reader,
                    &mut write_half,
                ).await;

                println!("'{request}' -> '{response}'");
            }
        });
    }
}


mod cli_player_client {
    const SCROLL_SENSITIVITY: f32 = 0.1;
    use rust_multiplayer::{
        app::client::{
            rendering::{
                renderer::State, 
                AppData
            }, 
            Client, 
            ClientHandle
        }, 
        requests::MoveDirection
        
    };

    use std::sync::{
        Arc, 
        Mutex
    };

    use winit::{
        application::ApplicationHandler, event::{
            ElementState, 
            WindowEvent
        }, 
        event_loop::{
            ActiveEventLoop, 
            ControlFlow, 
            EventLoop
        }, 
        keyboard::Key, 
        window::{
            Window, 
            WindowId
        }
    };

    #[derive(Default)]
    struct App {
        state: Option<State>,
        data: Arc<Mutex<AppData>>,
        client_handler: Option<ClientHandle>
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
                    match delta {
                        winit::event::MouseScrollDelta::LineDelta(_, y) => {
                            // y is +-1
                            if let Ok(mut app_data_guard) = self.data.lock() {
                                app_data_guard.scale *= (1.0 + SCROLL_SENSITIVITY).powf(y);
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


    pub fn run<A: tokio::net::ToSocketAddrs + std::fmt::Debug>(addr: A, player_name: &str) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            // wgpu uses `log` for all of our logging, so we initialize a logger with the `env_logger` crate.
            
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
            
            let client_handler = Client::connect(addr).await
                .run(app.data.clone()).await;

            app.client_handler = Some(client_handler);

            event_loop.run_app(&mut app).unwrap();

            // client_handler.clone().wait_until_finished().await.unwrap();
        });
    }
}