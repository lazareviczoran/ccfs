use actix_http::http::StatusCode;
use actix_web::{test, web, App};
use ccfs_commons::ChunkServer;
use chrono::{Duration, Utc};
use metadata_server::routes::api::{get_server, get_servers};
use metadata_server::ServersMap;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use test::{call_service, init_service, read_response_json, TestRequest};
use tokio::sync::RwLock;
use uuid::Uuid;

#[actix_rt::test]
async fn test_get_servers_no_active() -> std::io::Result<()> {
    let servers: ServersMap = Arc::new(RwLock::new(HashMap::new()));
    let server = init_service(
        App::new()
            .data(servers)
            .service(web::scope("/api").service(get_servers)),
    )
    .await;

    let req = TestRequest::get().uri("/api/servers").to_request();
    let data: Vec<ChunkServer> = read_response_json(&server, req).await;
    assert!(data.is_empty());
    Ok(())
}

#[actix_rt::test]
async fn test_get_servers_with_active() -> std::io::Result<()> {
    let mut map = HashMap::new();
    let s1_id = Uuid::from_str("1a6e7006-12a7-4935-b8c0-58fa7ea84b09").unwrap();
    let s2_id = Uuid::from_str("6d53a85f-505b-4a1a-ae6d-f7c18761d04a").unwrap();
    let mut s1 = ChunkServer::new(s1_id, "url1".into());
    s1.latest_ping_time = Utc::now() - Duration::seconds(10);
    let s2 = ChunkServer::new(s2_id, "url2".into());
    map.insert(s1_id, s1);
    map.insert(s2_id, s2.clone());
    let servers: ServersMap = Arc::new(RwLock::new(map));
    let server = init_service(
        App::new()
            .data(servers)
            .service(web::scope("/api").service(get_servers)),
    )
    .await;

    let req = TestRequest::get().uri("/api/servers").to_request();
    let data: Vec<ChunkServer> = read_response_json(&server, req).await;
    assert_eq!(data.len(), 1);
    assert_eq!(data[0].id, s2.id);
    Ok(())
}

#[actix_rt::test]
async fn test_get_single_server_missing() -> std::io::Result<()> {
    let servers: ServersMap = Arc::new(RwLock::new(HashMap::new()));
    let server = init_service(
        App::new()
            .data(servers)
            .service(web::scope("/api").service(get_server)),
    )
    .await;

    let req = TestRequest::get()
        .uri("/api/servers/1a6e7006-12a7-4935-b8c0-58fa7ea84b09")
        .to_request();
    let resp = call_service(&server, req).await;
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    Ok(())
}

#[actix_rt::test]
async fn test_get_single_server_success() -> std::io::Result<()> {
    let mut map = HashMap::new();
    let s1_id = Uuid::from_str("1a6e7006-12a7-4935-b8c0-58fa7ea84b09").unwrap();
    let s1 = ChunkServer::new(s1_id, "url1".into());
    map.insert(s1_id, s1);
    let servers: ServersMap = Arc::new(RwLock::new(map));
    let server = init_service(
        App::new()
            .data(servers)
            .service(web::scope("/api").service(get_server)),
    )
    .await;

    let req = TestRequest::get()
        .uri("/api/servers/1a6e7006-12a7-4935-b8c0-58fa7ea84b09")
        .to_request();
    let data: ChunkServer = read_response_json(&server, req).await;
    assert_eq!(data.id, s1_id);
    Ok(())
}
