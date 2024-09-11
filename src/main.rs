use std::env;
use tokio::runtime::Runtime;
mod cert;
mod client;
mod fileserver;
mod merkletree;

async fn run_server() {
    // run server
    let _ = fileserver::routes::start_server().await;
}

async fn run_client() {
    //run client
    let _ = client::routes::start_local_server().await;
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let rt = Runtime::new().unwrap();

    if args.len() < 2 {
        eprintln!("Usage: cargo run [client|server]");
        return;
    }

    match args[1].as_str() {
        "server" => {
            println!("Running the server on port 8080...");
            rt.block_on(run_server());
        }
        "client" => {
            println!("Running the client on port 8081...");
            rt.block_on(run_client());
        }
        _ => {
            eprintln!("Unknown argument: {}", args[1]);
            eprintln!("Usage: cargo run [client|server]");
        }
    }
}
