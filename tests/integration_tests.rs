use std::{
    sync::{
        Arc, 
        Mutex
    }, 
    time::Duration
};

use rust_multiplayer::{
    app::{
        client::{MultiplayerClient, MultiplayerClientHandle}, 
        server::{client_session::ClientSessionState, GameplayState, MultiplayerServer}
    }, game::world::PlayerRole, requests::{
        ClientRequest, ClientResponse, GameplayStateBrief, MoveDirection
    }
};

async fn run_single_client_test<F>(test_fn: F) 
where
    F: FnOnce(MultiplayerClientHandle) + Send + 'static,
{
    let server = MultiplayerServer::bind_any_local().await.unwrap();
    let server_address = server.get_local_address().unwrap();
    let server_handler = server.run().await.unwrap();
    assert_eq!(server_handler.connections_count(), 0);
    
    let client_offloaded_task = tokio::task::spawn_blocking(move || {
        let client = MultiplayerClient::connect(server_address).unwrap();
        let client_handler = client.run().unwrap();

        test_fn(client_handler);

        std::thread::sleep(Duration::from_millis(10));
    });
    
    server_handler.await_any_connection().await;
    assert_eq!(server_handler.connections_count(), 1, "Client not connected");

    client_offloaded_task.await.unwrap();

    server_handler.await_all_disconnect().await;
    assert_eq!(server_handler.connections_count(), 0, "Client not disconnected");

    server_handler.shutdown().await.unwrap();
}

struct MultipleClientsTestCfg {
    clients_count: usize,
    start_delay: core::ops::Range<Duration>,
    end_delay: core::ops::Range<Duration>,
}

async fn run_multiple_client_test<F>(
    multiple_clients_cfg: MultipleClientsTestCfg,
    test_fn: F
) 
where
    F: Fn(MultiplayerClientHandle) + Send + Sync + 'static
{
    assert!(multiple_clients_cfg.clients_count > 0);
    let server = MultiplayerServer::bind_any_local().await.unwrap();
    let server_address = server.get_local_address().unwrap();
    let server_handler = server.run().await.unwrap();
    assert_eq!(server_handler.connections_count(), 0);

    let test_fn = Arc::new(test_fn);
    
    let mut client_offloaded_tasks = vec![];

    for _ in 0..multiple_clients_cfg.clients_count {
        let test_fn_shared = test_fn.clone();
        let start_delay = rand::random_range(multiple_clients_cfg.start_delay.clone());
        let end_delay = rand::random_range(multiple_clients_cfg.end_delay.clone());

        let client_offloaded_task = tokio::task::spawn_blocking(move || {
        std::thread::sleep(start_delay);
            let client = MultiplayerClient::connect(server_address).unwrap();
            let client_handler = client.run().unwrap();

            test_fn_shared(client_handler);
        });
        std::thread::sleep(end_delay);
        client_offloaded_tasks.push(client_offloaded_task);
    }

    for client_offloaded_task in client_offloaded_tasks {
        client_offloaded_task.await.unwrap();
    }

    server_handler.await_all_disconnect().await;
    assert_eq!(server_handler.connections_count(), 0, "Client not disconnected");

    server_handler.shutdown().await.unwrap();
}


async fn run_multiple_client_single_final_client_test<F, P>(
    multiple_clients_cfg: MultipleClientsTestCfg,
    test_fn: F,
    test_final_fn: P
) 
where
    F: Fn(MultiplayerClientHandle) + Send + Sync + 'static,
    P: FnOnce(MultiplayerClientHandle) + Send + Sync + 'static,
{
    assert!(multiple_clients_cfg.clients_count > 0);
    let server = MultiplayerServer::bind_any_local().await.unwrap();
    let server_address = server.get_local_address().unwrap();
    let server_handler = server.run().await.unwrap();
    assert_eq!(server_handler.connections_count(), 0);

    let test_fn = Arc::new(test_fn);
    
    let mut client_offloaded_tasks = vec![];

    // Spawn multiple clients
    for _ in 0..multiple_clients_cfg.clients_count {
        let test_fn_shared = test_fn.clone();
        let start_delay = rand::random_range(multiple_clients_cfg.start_delay.clone());
        let end_delay = rand::random_range(multiple_clients_cfg.end_delay.clone());

        let client_offloaded_task = tokio::task::spawn_blocking(move || {
            std::thread::sleep(start_delay);
            let client = MultiplayerClient::connect(server_address).unwrap();
            let client_handler = client.run().unwrap();

            test_fn_shared(client_handler);
            std::thread::sleep(end_delay);
        });
        client_offloaded_tasks.push(client_offloaded_task);
    }

    for client_offloaded_task in client_offloaded_tasks {
        client_offloaded_task.await.unwrap();
    }

    tokio::time::sleep(Duration::from_millis(10)).await;
    server_handler.await_all_disconnect().await; // Monitor, once failed. Added delay before, meybe will help.
    assert_eq!(server_handler.connections_count(), 0, "Client not disconnected");

    // Spawn final client
    let client_offloaded_task = tokio::task::spawn_blocking(move || {
        let client = MultiplayerClient::connect(server_address).unwrap();
        let client_handler = client.run().unwrap();

        test_final_fn(client_handler);

        std::thread::sleep(Duration::from_millis(10));
    });

    client_offloaded_task.await.unwrap();

    server_handler.await_all_disconnect().await;
    assert_eq!(server_handler.connections_count(), 0, "Client not disconnected");

    server_handler.shutdown().await.unwrap();
}

fn wait_until_game_started(client_handler: &MultiplayerClientHandle) {
    loop {
        std::thread::sleep(Duration::from_millis(250));
        let response = client_handler.make_request_with_timeout(ClientRequest::GetStartCountdownTime, None).unwrap();
        if matches!(response, ClientResponse::BadState) {
            break;
        }
    }
}

#[tokio::test]
async fn test_client_connect_disconnect_on_their_own() {
    run_single_client_test(|client_handler| {
        let response = client_handler.make_request_with_timeout(ClientRequest::ServerCheck, None).unwrap();
        matches!(response, ClientResponse::ServerCheck { msg: _, connections: 1 });
    }).await;
}

#[tokio::test]
async fn test_client_common_read_only_requests() {
    run_single_client_test(|client_handler| {
        let response = client_handler.make_request_with_timeout(ClientRequest::GetClientSessionData, None).unwrap();
        match response {
            ClientResponse::GetClientSessionData { data } => {
                assert_eq!(data.state, ClientSessionState::JustConnected);
                assert_eq!(data.get_entity_player_id(), None);
            },
            _ => panic!("Bad response={response:?}"),
        }

        let response = client_handler.make_request_with_timeout(ClientRequest::GetClientSessionId, None).unwrap();
        matches!(response, ClientResponse::GetClientSessionId { id: _ });
        
        let response = client_handler.make_request_with_timeout(ClientRequest::ServerCheck, None).unwrap();
        matches!(response, ClientResponse::ServerCheck { msg: _, connections: 1 });
        
        let response = client_handler.make_request(ClientRequest::GetEntityId).unwrap();
        assert!(matches!(response, ClientResponse::GetEntityId { id: None }), "{response:?}");
        
        let response = client_handler.make_request(ClientRequest::WorldCheck).unwrap();
        assert!(matches!(response, ClientResponse::BadState), "{response:?}");
        
        let response = client_handler.make_request(ClientRequest::Move { dir: MoveDirection::Down }).unwrap();
        assert!(matches!(response, ClientResponse::BadState), "{response:?}");
    }).await;
}

#[tokio::test]
async fn test_client_set_name() {
    run_single_client_test(|client_handler| {
    let name_to_be_set = "Famcyname101";
        let response = client_handler.make_request_with_timeout(ClientRequest::GetClientSessionData, None).unwrap();
        match response {
            ClientResponse::GetClientSessionData { data } => {
                assert_eq!(data.state, ClientSessionState::JustConnected);
                assert_eq!(data.get_entity_player_id(), None);
                assert_eq!(data.get_name(), None);
            },
            _ => panic!("Bad response={response:?}"),
        }

        let response = client_handler.make_request_with_timeout(ClientRequest::SetName { new_name: Some(name_to_be_set.to_string()) }, None).unwrap();
        match response {
            ClientResponse::SetName { result } => {
                assert!(result.is_ok());
            },
            _ => panic!("Bad response={response:?}"),
        }

        let response = client_handler.make_request_with_timeout(ClientRequest::GetClientSessionData, None).unwrap();
        match response {
            ClientResponse::GetClientSessionData { data } => {
                assert!(matches!(data.state, ClientSessionState::NameWasSet { name: _, ready_to_start: false, entity_player_id: None }));
                assert_eq!(data.get_name(), Some(name_to_be_set));
            },
            _ => panic!("Bad response={response:?}"),
        }
    }).await;
}

#[tokio::test]
async fn test_client_set_ready() {
    run_single_client_test(|client_handler| {
        let name_to_be_set = "Famcyname101";
        let response = client_handler.make_request_with_timeout(ClientRequest::SetName { new_name: Some(name_to_be_set.to_string()) }, None).unwrap();
        match response {
            ClientResponse::SetName { result } => {
                assert!(result.is_ok());
            },
            _ => panic!("Bad response={response:?}"),
        }

        let response = client_handler.make_request_with_timeout(ClientRequest::SetReady { ready: true }, None).unwrap();
        match response {
            ClientResponse::SetReady { was_set } => {
                assert!(was_set);
            },
            _ => panic!("Bad response={response:?}"),
        }

        let response = client_handler.make_request_with_timeout(ClientRequest::GetClientSessionData, None).unwrap();
        match response {
            ClientResponse::GetClientSessionData { data } => {
                assert!(matches!(data.state, ClientSessionState::NameWasSet { name: _, ready_to_start: true, entity_player_id: None }));
                assert_eq!(data.get_entity_player_id(), None);
                assert_eq!(data.get_name(), Some(name_to_be_set));
            },
            _ => panic!("Bad response={response:?}"),
        }
    }).await;
}

#[tokio::test]
async fn test_client_ping_server() {
    run_single_client_test(|client_handler| {
        let ping_result = client_handler.ping(10, Duration::from_micros(500), None, Duration::from_millis(10));
        println!("{:?}", ping_result);
    }).await;
}

#[tokio::test]
async fn test_new_client_has_no_points() {
    run_single_client_test(|client_handler| {
        let response = client_handler.make_request_with_timeout(ClientRequest::GetPointsCount, None).unwrap();
        match response {
            ClientResponse::GetPointsCount { points_count } => {
                assert_eq!(points_count, 0);
            },
            _ => panic!("Bad response={response:?}"),
        }
    }).await;
}

#[tokio::test]
async fn test_client_gets_generated_name() {
    run_single_client_test(|client_handler| {
        let response = client_handler.make_request_with_timeout(ClientRequest::SetName { new_name: None }, None).unwrap();
        match response {
            ClientResponse::SetName { result } => {
                assert!(result.is_ok());
            },
            _ => panic!("Bad response={response:?}"),
        }

        let response = client_handler.make_request_with_timeout(ClientRequest::GetClientSessionData, None).unwrap();
        match response {
            ClientResponse::GetClientSessionData { data } => match data.state {
                ClientSessionState::NameWasSet { name, ready_to_start, entity_player_id } => {
                    assert!(!name.is_empty());
                    assert!(!ready_to_start);
                    assert!(entity_player_id.is_none());
                    println!("{name}");
                },
                _ => panic!("Bad state"),
            },
            _ => panic!("Bad response={response:?}"),
        };
    }).await;
}

#[tokio::test]
async fn test_multiple_clients() {
    let config = MultipleClientsTestCfg {
        clients_count: 100,
        start_delay: Duration::from_micros(0)..Duration::from_micros(2),
        end_delay: Duration::from_micros(0)..Duration::from_micros(2),
    };

    run_multiple_client_test(config, |client_handler| {
        let response = client_handler.make_request_with_timeout(ClientRequest::SetName { new_name: None }, None).unwrap();
        match response {
            ClientResponse::SetName { result } => {
                assert!(result.is_ok());
            },
            _ => panic!("Bad response={response:?}"),
        }

        let response = client_handler.make_request_with_timeout(ClientRequest::GetClientSessionData, None).unwrap();
        match response {
            ClientResponse::GetClientSessionData { data } => {
                assert!(data.get_name().is_some());
                println!("{}", data.get_name().unwrap());
            },
            _ => panic!("Bad response={response:?}"),
        }

        let response = client_handler.make_request_with_timeout(ClientRequest::GetPointsCount, None).unwrap();
        match response {
            ClientResponse::GetPointsCount { points_count } => {
                assert_eq!(points_count, 0);
            },
            _ => panic!("Bad response={response:?}"),
        };
    }).await;
}

#[tokio::test]
async fn test_multiple_clients_chatting() {
    const CLIENTS_COUNT: usize = 10;
    let config = MultipleClientsTestCfg {
        clients_count: CLIENTS_COUNT,
        start_delay: Duration::from_micros(0)..Duration::from_micros(2),
        end_delay: Duration::from_micros(0)..Duration::from_micros(2),
    };

    let counter = Arc::new(Mutex::new(0));
    let counter_shared = counter.clone();

    let per_client_test_fn = move |client_handler: MultiplayerClientHandle| {
        let counter_shared_clone = counter_shared.clone();
        let response = client_handler.make_request_with_timeout(ClientRequest::SetName { new_name: None }, None).unwrap();
        match response {
            ClientResponse::SetName { result } => {
                assert!(result.is_ok());
            },
            _ => panic!("Bad response={response:?}"),
        }

        let this_counter = {
            let mut guard = counter_shared_clone.lock().unwrap();
            *guard += 1;
            *guard
        };

        let response = client_handler.make_request_with_timeout(ClientRequest::SendChatMessage { msg: format!("Hello message '{this_counter}'!") }, None).unwrap();
        match response {
            ClientResponse::SendChatMessage { sent } => {
                assert!(sent);
            },
            _ => panic!("Bad response={response:?}"),
        }

        println!("Client test_fn ran. Counter: {}", this_counter);
    };

    run_multiple_client_single_final_client_test(
        config, 
        per_client_test_fn,
        |final_client_handler| {
            let response = final_client_handler.make_request_with_timeout(ClientRequest::ReadChatMessages { max_count: None }, None).unwrap();
            match response {
                ClientResponse::ReadChatMessages { results } => {
                    assert!(results.len() > CLIENTS_COUNT);

                    for msg in results {
                        println!("{msg}");
                    }
                },
                _ => panic!("Bad response={response:?}"),
            }

            let expected_messages = 3;
            assert!(expected_messages < CLIENTS_COUNT);
            let response = final_client_handler.make_request_with_timeout(ClientRequest::ReadChatMessages { max_count: Some(expected_messages) }, None).unwrap();
            match response {
                ClientResponse::ReadChatMessages { results } => {
                    assert_eq!(results.len(), expected_messages);
                },
                _ => panic!("Bad response={response:?}"),
            }
        }
    ).await;

    assert_eq!(*counter.lock().unwrap(), CLIENTS_COUNT);
}

#[tokio::test]
async fn test_check_initial_gameplay_state_should_be_lobby() {
    run_single_client_test(|client_handler| {
        let response = client_handler.make_request_with_timeout(ClientRequest::CheckGameplayState, None).unwrap();
        match response {
            ClientResponse::CheckGameplayState { state } => {
                assert!(matches!(state, GameplayStateBrief::Lobby { counting_to_start: None, last_result: _ }));
            },
            _ => panic!("Bad response={response:?}"),
        }
    }).await;
}

#[tokio::test]
async fn test_multiple_clients_getting_ready_trigger_countdown_await_expiration() {
    let clients_count = 3;
    let config = MultipleClientsTestCfg {
        clients_count,
        start_delay: Duration::from_micros(0)..Duration::from_micros(2),
        end_delay: Duration::from_micros(0)..Duration::from_micros(2),
    };

    let players_roles = Arc::new(Mutex::new(Vec::new()));
    let players_roles_clonned = players_roles.clone();

    let test_every_client = move |client_handler: MultiplayerClientHandle| {
        let response = client_handler.make_request_with_timeout(ClientRequest::SetName { new_name: None }, None).unwrap();
        match response {
            ClientResponse::SetName { result } => {
                assert!(result.is_ok());
            },
            _ => panic!("Bad response={response:?}"),
        }
        
        // Counting should not happen before being ready
        let response = client_handler.make_request_with_timeout(ClientRequest::GetStartCountdownTime, None).unwrap();
        match response {
            ClientResponse::GetStartCountdownTime { time } => {
                assert!(time.is_none());
            },
            _ => panic!("Bad response={response:?}"),
        };

        // Set all ready to start bounting
        let response = client_handler.make_request_with_timeout(ClientRequest::SetReady { ready: true }, None).unwrap();
        match response {
            ClientResponse::SetReady { was_set } => {
                assert!(was_set);
            },
            _ => panic!("Bad response={response:?}"),
        }

        // Delay shorter than counting expiration
        std::thread::sleep(Duration::from_millis(100));

        // Here counting down should be running
        let response = client_handler.make_request_with_timeout(ClientRequest::GetStartCountdownTime, None).unwrap();
        match response {
            ClientResponse::GetStartCountdownTime { time } => {
                assert!(time.is_some());
                println!("{:?}", time);
            },
            _ => panic!("Bad response={response:?}"),
        };

        // Delay long enough to expire countdown
        wait_until_game_started(&client_handler);

        // Players should be ingame now, check what role they have
        let response = client_handler.make_request_with_timeout(ClientRequest::GetRole, None).unwrap();
        match response {
            ClientResponse::GetRole { role } => {
                println!("{:?}", role);
                let mut players_roles_guard = players_roles_clonned.lock().unwrap();
                players_roles_guard.push(role);
            },
            _ => panic!("Bad response={response:?}"),
        };

        // Check gameplay state, must be in GameRunning
        let response = client_handler.make_request_with_timeout(ClientRequest::CheckGameplayState, None).unwrap();
        match response {
            ClientResponse::CheckGameplayState { state } => {
                assert!(matches!(state, GameplayStateBrief::GameRunning));
            },
            _ => panic!("Bad response={response:?}"),
        };

        // Each playermust see at least 'clients_count' entities
        let response = client_handler.make_request_with_timeout(ClientRequest::WorldCheck, None).unwrap();
        match response {
            ClientResponse::WorldCheck { entities } => {
                assert!(entities.len() >= clients_count);
                println!("entities.len={}", entities.len());
            },
            _ => panic!("Bad response={response:?}"),
        };
    };

    run_multiple_client_test(config, test_every_client).await;

    // Check summary
    let players_roles_guard = players_roles.lock().unwrap();
    assert_eq!(players_roles_guard.len(), clients_count);
    
    let seekers_count = players_roles_guard.iter()
        .filter(|role| matches!(role, PlayerRole::Seeker { stats: _}))
        .count();
    assert_eq!(seekers_count, 1);
}


#[tokio::test]
async fn test_server_drops_all_connetions() {
    let server = MultiplayerServer::bind_any_local().await.unwrap();
    let server_handler = server.run().await.unwrap();
    assert_eq!(server_handler.connections_count(), 0);
    // TODO
}
