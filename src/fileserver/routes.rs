use super::fs::{handle_file_download, handle_file_upload};
use crate::fileserver::fs::list_files_handler;
use std::fs::{self};
use std::sync::Arc;
use warp::Filter;

const UPLOAD_DIR: &str = "filestore/";

pub async fn start_server() {
    fs::create_dir_all(UPLOAD_DIR).unwrap();
    let db = sled::open("merkle_tree_db").expect("Failed to open database");

    // put inside Arc for shared ownership
    let db = Arc::new(db);
    let db_filter = warp::any().map(move || Arc::clone(&db));

    let upload_page = warp::path("upload")
        .and(warp::get())
        .and(warp::fs::file("./static/upload.html"));

    let upload_route = warp::path("upload")
        .and(db_filter.clone())
        .and(warp::post())
        .and(warp::multipart::form().max_length(10_000_000))
        .and_then(handle_file_upload);

    let download_route = warp::path("download")
        .and(db_filter.clone())
        .and(warp::path::param::<String>())
        .and_then(handle_file_download);

    let list_page = warp::path("list").and(warp::fs::file("./static/list.html"));

    let download_page = warp::path("downloads").and(warp::fs::file("./static/download.html"));

    let list_files = warp::path("files")
        .and(warp::get())
        .and_then(list_files_handler);

    let routes = list_page
        .or(list_files)
        .or(upload_page)
        .or(upload_route)
        .or(download_page)
        .or(download_route);
    // Start the server

    warp::serve(routes).run(([127, 0, 0, 1], 8080)).await;
}
