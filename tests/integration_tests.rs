use std::time::Duration;

use rust_multiplayer::{
    app::{
        client::MultiplayerClient, 
        server::{client_session::{ClientSessionState, GameplayState}, MultiplayerServer}
    }, requests::{
        ClientRequest, 
        ClientResponse, MoveDirection
    }
};

#[tokio::test]
async fn test_client_connect_disconnect_on_their_own() {
    let server = MultiplayerServer::bind_any_local().await.unwrap();
    let server_address = server.get_local_address().unwrap();
    let server_handler = server.run().await.unwrap();
    assert_eq!(server_handler.connections_count(), 0);

    let client_thread = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let client = MultiplayerClient::connect(server_address).unwrap();
            let client_handler = client.run().unwrap();
            let response = client_handler.make_request_with_timeout(ClientRequest::ServerCheck, None).unwrap();
            matches!(response, ClientResponse::ServerCheck { msg: _, connections: 1 });
            tokio::time::sleep(Duration::from_millis(10)).await;
        });
    });
    
    server_handler.await_any_connection().await;
    assert_eq!(server_handler.connections_count(), 1, "Client not connected");

    server_handler.await_all_disconnect().await;
    assert_eq!(server_handler.connections_count(), 0, "Client not disconnected");
    client_thread.join().unwrap();
    server_handler.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_client_common_read_only_requests() {
    let server = MultiplayerServer::bind_any_local().await.unwrap();
    let server_address = server.get_local_address().unwrap();
    let server_handler = server.run().await.unwrap();

    let client_thread = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let client = MultiplayerClient::connect(server_address).unwrap();
            let client_handler = client.run().unwrap();
            
            let response = client_handler.make_request_with_timeout(ClientRequest::GetClientSessionData, None).unwrap();
            match response {
                ClientResponse::GetClientSessionData { data } => {
                    assert_eq!(data.state, ClientSessionState::JustConnected);
                    assert_eq!(data.get_entity_player_id(), None);
                },
                _ => panic!("Bad response"),
            }

            let response = client_handler.make_request_with_timeout(ClientRequest::GetClientSessionId, None).unwrap();
            matches!(response, ClientResponse::GetClientSessionId { id: _ });
            
            let response = client_handler.make_request_with_timeout(ClientRequest::ServerCheck, None).unwrap();
            matches!(response, ClientResponse::ServerCheck { msg: _, connections: 1 });
            
            let response = client_handler.make_request(ClientRequest::GetEntityId).unwrap();
            matches!(response, ClientResponse::GetEntityId { id: None });
            
            let response = client_handler.make_request(ClientRequest::WorldCheck).unwrap();
            match response {
                ClientResponse::WorldCheck { entities } => assert!(entities.is_empty()),
                _ => panic!("Bad response"),
            }
            
            let response = client_handler.make_request(ClientRequest::Move { dir: MoveDirection::Down }).unwrap();
            match response {
                ClientResponse::Move { started } => assert!(!started),
                _ => panic!("Bad response"),
            }

            tokio::time::sleep(Duration::from_millis(10)).await;
        });
    });
    
    server_handler.await_any_connection().await;
    server_handler.await_all_disconnect().await;

    client_thread.join().unwrap();
    server_handler.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_client_set_name() {
    let server = MultiplayerServer::bind_any_local().await.unwrap();
    let server_address = server.get_local_address().unwrap();
    let server_handler = server.run().await.unwrap();

    let name_to_be_set = "Famcyname101";

    let client_thread = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let client = MultiplayerClient::connect(server_address).unwrap();
            let client_handler = client.run().unwrap();
            
            let response = client_handler.make_request_with_timeout(ClientRequest::GetClientSessionData, None).unwrap();
            match response {
                ClientResponse::GetClientSessionData { data } => {
                    assert_eq!(data.state, ClientSessionState::JustConnected);
                    assert_eq!(data.get_entity_player_id(), None);
                    assert_eq!(data.get_name(), None);
                },
                _ => panic!("Bad response"),
            }

            let response = client_handler.make_request_with_timeout(ClientRequest::SetName { new_name: Some(name_to_be_set.to_string()) }, None).unwrap();
            match response {
                ClientResponse::SetName { result } => {
                    assert!(result.is_ok());
                },
                _ => panic!("Bad response"),
            }

            let response = client_handler.make_request_with_timeout(ClientRequest::GetClientSessionData, None).unwrap();
            match response {
                ClientResponse::GetClientSessionData { data } => {
                    assert!(matches!(data.state, ClientSessionState::NameWasSet { name: _, gameplay_state: GameplayState::Lobby {ready: _} }));
                    assert_eq!(data.get_entity_player_id(), None);
                    assert_eq!(data.get_name(), Some(name_to_be_set));
                },
                _ => panic!("Bad response"),
            }

            tokio::time::sleep(Duration::from_millis(10)).await;
        });
    });
    
    server_handler.await_any_connection().await;
    server_handler.await_all_disconnect().await;

    client_thread.join().unwrap();
    server_handler.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_client_set_ready() {
    let server = MultiplayerServer::bind_any_local().await.unwrap();
    let server_address = server.get_local_address().unwrap();
    let server_handler = server.run().await.unwrap();

    let name_to_be_set = "Famcyname101";

    let client_thread = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let client = MultiplayerClient::connect(server_address).unwrap();
            let client_handler = client.run().unwrap();

            let response = client_handler.make_request_with_timeout(ClientRequest::SetName { new_name: Some(name_to_be_set.to_string()) }, None).unwrap();
            match response {
                ClientResponse::SetName { result } => {
                    assert!(result.is_ok());
                },
                _ => panic!("Bad response"),
            }

            let response = client_handler.make_request_with_timeout(ClientRequest::SetReady { ready: true }, None).unwrap();
            match response {
                ClientResponse::SetReady { was_set } => {
                    assert!(was_set);
                },
                _ => panic!("Bad response"),
            }

            let response = client_handler.make_request_with_timeout(ClientRequest::GetClientSessionData, None).unwrap();
            match response {
                ClientResponse::GetClientSessionData { data } => {
                    assert!(matches!(data.state, ClientSessionState::NameWasSet { name: _, gameplay_state: GameplayState::Lobby {ready: true} }));
                    assert_eq!(data.get_entity_player_id(), None);
                    assert_eq!(data.get_name(), Some(name_to_be_set));
                },
                _ => panic!("Bad response"),
            }

            tokio::time::sleep(Duration::from_millis(10)).await;
        });
    });
    
    server_handler.await_any_connection().await;
    server_handler.await_all_disconnect().await;

    client_thread.join().unwrap();
    server_handler.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_client_ping_server() {
    let server = MultiplayerServer::bind_any_local().await.unwrap();
    let server_address = server.get_local_address().unwrap();
    let server_handler = server.run().await.unwrap();

    let client_thread = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let client = MultiplayerClient::connect(server_address).unwrap();
            let client_handler = client.run().unwrap();

            let ping_result = client_handler.ping(10, Duration::from_micros(500), None, Duration::from_millis(10));
            println!("{:?}", ping_result);

            tokio::time::sleep(Duration::from_millis(10)).await;
        });
    });
    
    server_handler.await_any_connection().await;
    server_handler.await_all_disconnect().await;

    client_thread.join().unwrap();
    server_handler.shutdown().await.unwrap();
}
#[tokio::test]
async fn test_new_client_has_no_points() {
    let server = MultiplayerServer::bind_any_local().await.unwrap();
    let server_address = server.get_local_address().unwrap();
    let server_handler = server.run().await.unwrap();

    let client_thread = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let client = MultiplayerClient::connect(server_address).unwrap();
            let client_handler = client.run().unwrap();

            let response = client_handler.make_request_with_timeout(ClientRequest::GetPointsCount, None).unwrap();
            match response {
                ClientResponse::GetPointsCount { points_count } => {
                    assert_eq!(points_count, 0);
                },
                _ => panic!("Bad response"),
            }

            tokio::time::sleep(Duration::from_millis(10)).await;
        });
    });
    
    server_handler.await_any_connection().await;
    server_handler.await_all_disconnect().await;

    client_thread.join().unwrap();
    server_handler.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_client_gets_generated_name() {
    let server = MultiplayerServer::bind_any_local().await.unwrap();
    let server_address = server.get_local_address().unwrap();
    let server_handler = server.run().await.unwrap();

    let client_thread = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let client = MultiplayerClient::connect(server_address).unwrap();
            let client_handler = client.run().unwrap();

            let response = client_handler.make_request_with_timeout(ClientRequest::SetName { new_name: None }, None).unwrap();
            match response {
                ClientResponse::SetName { result } => {
                    assert!(result.is_ok());
                },
                _ => panic!("Bad response"),
            }

            let response = client_handler.make_request_with_timeout(ClientRequest::GetClientSessionData, None).unwrap();
            match response {
                ClientResponse::GetClientSessionData { data } => match data.state {
                    ClientSessionState::NameWasSet { name, gameplay_state } => {
                        assert!(matches!(gameplay_state, GameplayState::Lobby { ready: _ }));
                        assert!(!name.is_empty());
                        println!("{name}");
                    },
                    _ => panic!("Bad state"),
                },
                _ => panic!("Bad response"),
            }

            tokio::time::sleep(Duration::from_millis(10)).await;
        });
    });
    
    server_handler.await_any_connection().await;
    server_handler.await_all_disconnect().await;

    client_thread.join().unwrap();
    server_handler.shutdown().await.unwrap();
}


#[tokio::test]
async fn test_server_drops_all_connetions() {
    let server = MultiplayerServer::bind_any_local().await.unwrap();
    let server_handler = server.run().await.unwrap();
    assert_eq!(server_handler.connections_count(), 0);
    // TODO
}
