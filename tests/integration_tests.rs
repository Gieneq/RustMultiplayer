use std::time::Duration;

use rust_multiplayer::{app::{client::{MultiplayerClient}, server::MultiplayerServer}, requests::{ClientRequest, ClientResponse}};


#[tokio::test]
async fn test_clients_connect_disconnect_on_their_own() {
    let server = MultiplayerServer::bind_any_local().await.unwrap();
    let server_address = server.get_local_address().unwrap();
    let server_handler = server.run().await.unwrap();
    assert_eq!(server_handler.connections_count(), 0);

    let client_thread = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let client = MultiplayerClient::connect(server_address).unwrap();
            let client_handler = client.run().unwrap();
            let response = client_handler.make_request_with_timeout(ClientRequest::Healthcheck, None).unwrap();
            matches!(response, ClientResponse::Healthcheck { msg: _, connections: 1 });
            tokio::time::sleep(Duration::from_millis(10)).await;
        });
    });
    
    tokio::time::sleep(Duration::from_millis(5)).await;
    assert_eq!(server_handler.connections_count(), 1, "Client not connected");
    client_thread.join().unwrap();
    tokio::time::sleep(Duration::from_millis(10)).await;
    assert_eq!(server_handler.connections_count(), 0, "Client not disconnected");
    server_handler.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_server_drops_all_connetions() {
    let server = MultiplayerServer::bind_any_local().await.unwrap();
    let server_handler = server.run().await.unwrap();
    assert_eq!(server_handler.connections_count(), 0);
}
