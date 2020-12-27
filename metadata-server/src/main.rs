#[macro_use]
extern crate rocket;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate rocket_contrib;
extern crate ccfs_commons;
extern crate mut_static;

use ccfs_commons::{Chunk, ChunkServer, File, FileStatus};
use mut_static::MutStatic;
use rocket_contrib::json::{Json, JsonValue};
use rocket_contrib::uuid::uuid_crate as uuid;
use rocket_contrib::uuid::Uuid;
use std::collections::HashMap;
use std::time::Instant;

lazy_static! {
    // should be replaced with DB
    static ref CHUNK_SERVERS:
        MutStatic<HashMap<uuid::Uuid, ChunkServer>> = MutStatic::new();
    static ref FILES:
        MutStatic<HashMap<uuid::Uuid, File>> = MutStatic::new();
    static ref CHUNKS:
        MutStatic<HashMap<uuid::Uuid, Chunk>> = MutStatic::new();
}

/// Returns a list of available chunk servers where the file chunks can be uploaded
#[get("/servers")]
fn get_servers() -> JsonValue {
    let servers_map = CHUNK_SERVERS.read().unwrap();
    let server_refs: Vec<ChunkServer> = servers_map
        .iter()
        .map(|(_, server)| server.clone())
        .collect();
    json!(server_refs)
}

/// Returns a list of available chunk servers where the file chunks
/// can be uploaded
#[get("/servers/<id>")]
fn get_server(id: Uuid) -> JsonValue {
    let servers_map = CHUNK_SERVERS.read().unwrap();
    if let Some(server) = servers_map.get(&id) {
        json!(server)
    } else {
        let reason = format!("Could not find server with ID {}", id);
        json!({
          "status": "error",
          "reason": reason
        })
    }
}

/// Registers a chunk server as active, or updates the latest_ping_time
/// if the map already contains it
#[post("/ping")]
fn chunk_server_ping(header_info: ChunkServer) -> JsonValue {
    let server_id = header_info.id;
    let server_addr = header_info.address;

    let mut servers_map = CHUNK_SERVERS.write().unwrap();
    let chunk_server;
    if let Some(server) = servers_map.get_mut(&server_id) {
        server.latest_ping_time = Instant::now();
        json!(*server)
    } else {
        chunk_server = ChunkServer::new(server_id, server_addr);
        let resp = json!(chunk_server);
        servers_map.insert(server_id.into_inner(), chunk_server);
        resp
    }
}

/// Creates a file entity with basic file info
#[post("/files/upload", format = "json", data = "<file_info>")]
fn create_file(file_info: Json<File>) -> JsonValue {
    let name = file_info.0.name;
    let size = file_info.0.size;
    let file = File::new(name, size);
    let resp = json!(file);
    let mut files_map = FILES.write().unwrap();
    files_map.insert(file.id.into_inner(), file);

    resp
}

/// Returns the file info
#[get("/files/<id>")]
fn get_file(id: Uuid) -> JsonValue {
    let files_map = FILES.read().unwrap();
    let file = files_map.get(&id);

    json!(*file.unwrap())
}

/// Because Uuid implements the Deref trait, we use Deref coercion to convert
/// rocket_contrib::uuid::Uuid to uuid::Uuid.
/// Notifies the metadata server to mark the chunk as completed
#[post("/chunk/completed", format = "json", data = "<chunk_info>")]
fn signal_chuck_upload_completed(chunk_info: Json<Chunk>) -> JsonValue {
    let mut chunks_map = CHUNKS.write().unwrap();

    let mut files_map = FILES.write().unwrap();
    if let Some(file) = files_map.get_mut(&chunk_info.0.file_id) {
        // The creation of the chunk entity should be actually in the chunk
        // server api, but for it is also here for dev purpose
        let chunk = chunk_info.into_inner();
        chunks_map.insert(chunk.id.into_inner(), chunk);

        file.num_of_completed_chunks += 1;
        if file.num_of_completed_chunks == file.num_of_chunks {
            file.status = FileStatus::Completed;
        }
        json!(file)
    } else {
        let reason = format!("Could not find file with ID {}", chunk_info.0.file_id);
        json!({
            "status": "error",
            "reason": reason
        })
    }
}

/// Because Uuid implements the Deref trait, we use Deref coercion to convert
/// rocket_contrib::uuid::Uuid to uuid::Uuid.
/// Returns the list of servers which contain the
/// uploaded chunks for a file
#[get("/chunks/file/<file_id>")]
fn get_chunks(file_id: Uuid) -> JsonValue {
    let chunks_map = CHUNKS.read().unwrap();
    let mut chunk_refs: Vec<Chunk> = chunks_map.iter().map(|(_, chunk)| *chunk).collect();
    chunk_refs.retain(|chunk| chunk.file_id == *file_id);
    json!(chunk_refs)
}

#[catch(404)]
fn not_found() -> JsonValue {
    json!({
        "status": "error",
        "reason": "Resource was not found."
    })
}

#[launch]
fn rocket() -> rocket::Rocket {
    CHUNK_SERVERS.set(HashMap::new()).unwrap();
    FILES.set(HashMap::new()).unwrap();
    CHUNKS.set(HashMap::new()).unwrap();
    rocket::ignite()
        .mount(
            "/api",
            routes![
                get_servers,
                get_server,
                chunk_server_ping,
                create_file,
                signal_chuck_upload_completed,
                get_file,
                get_chunks
            ],
        )
        .register(catchers![not_found])
}
