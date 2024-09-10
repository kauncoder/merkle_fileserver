use warp::filters::multipart::FormData;
use warp::reject::Rejection;
use warp::reply::Reply;
use std::convert::Infallible;
use std::fs::{self};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use futures::TryStreamExt;
use warp::{self, Buf,http::StatusCode};
use crate::merkletree::tree::FastMerkleTree;

const UPLOAD_DIR: &str = "filestore/";

pub async fn handle_file_upload(db: Arc<sled::Db>,mut form: FormData) -> Result<impl warp::Reply, Infallible> {

    while let Ok(Some(part)) = form.try_next().await {
        let filename = match part.filename() {
            Some(filename) => filename.to_string(),
            None => return Ok(StatusCode::BAD_REQUEST),
        };
        let mut data = Vec::new();
        let mut stream = part.stream();
        
        while let Ok(Some(chunk)) = stream.try_next().await {
            data.extend_from_slice(chunk.chunk());
        }

        let save_path = PathBuf::from(format!("./{}{}",UPLOAD_DIR, filename));
        if let Err(_) = tokio::fs::write(save_path, data).await {
            return Ok(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    //build merkle tree for the entire upload folder
    let file_hash_map = get_file_list();
    FastMerkleTree::build_merkle_tree (db,file_hash_map);
    Ok(StatusCode::CREATED)


}

pub async fn handle_file_download(db: Arc<sled::Db>,filename: String) -> Result<impl Reply, Rejection> {
    use tokio_util::io::ReaderStream;
    let filepath = PathBuf::from(format!("./filestore/{}", filename));
    if filepath.exists() {
        let mut merkle_proof : Vec<(Vec<u8>,bool)> = Vec::new();

        // get merkle proof from db
       if let Some(proof) = FastMerkleTree::get_merkle_proof_from_db(db, format!("./filestore/{}", filename)){
        merkle_proof = proof;
       };
        let file = tokio::fs::File::open(filepath).await.unwrap();

        let stream = ReaderStream::new(file);
        let response = warp::http::response::Builder::new()
        .header("Content-Disposition",format!("attachment; filename=\"{}\"", filename))
        .header("X-File-Hash",format!("{:?}",merkle_proof))
        .body(warp::hyper::Body::wrap_stream(stream))
        .unwrap();
        Ok(response)
    } else {
        Err(warp::reject::not_found())
    }
}

// Handler to list files
pub async fn list_files_handler() -> Result<impl Reply, Rejection> {
    // Read directory contents
    let mut files = vec![];
    if let Ok(entries) = fs::read_dir(UPLOAD_DIR) {
        for entry in entries {
            if let Ok(entry) = entry {
                if let Ok(filename) = entry.file_name().into_string() {
                    files.push(filename);
                }
            }
        }
    }
    // Respond with list of files
    Ok(warp::reply::json(&files))
}

fn get_file_list()->Vec<String>{
    let mut file_list :Vec<String> = Vec::new();    //replace with more concrete type
    let dir_path = format!("./{}",UPLOAD_DIR);
     let dir = Path::new(&dir_path);
     let entries = fs::read_dir(dir).unwrap();
     for entry in entries{
        let path = entry.unwrap().path();
        file_list.push(path.display().to_string())
     }
     file_list
}