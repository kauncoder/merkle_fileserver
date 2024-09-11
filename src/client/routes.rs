use super::client::{handle_file_hash, handle_verify};
use warp::Filter;

pub async fn start_local_server() {
    let hash_page = warp::path("hash")
        .and(warp::get())
        .and(warp::fs::file("./static/hash.html"));

    let hash_route = warp::path("hashform")
        .and(warp::post())
        .and(warp::multipart::form().max_length(10_000_000))
        .and_then(handle_file_hash);

    let verify_page = warp::path("verify")
        .and(warp::get())
        .and(warp::fs::file("./static/verify.html"));

    let verify_route = warp::path("verifyform")
        .and(warp::post())
        .and(warp::multipart::form().max_length(10_000_000))
        .and_then(handle_verify);

    let routes = hash_page.or(hash_route).or(verify_page).or(verify_route); //.or(static_files);
                                                                            // Start the server
    warp::serve(routes).run(([127, 0, 0, 1], 8081)).await;
}
