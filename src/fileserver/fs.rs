use crate::merkletree::tree::FastMerkleTree;
use anyhow::Result;
use futures::TryStreamExt;
use regex::Regex;
use std::convert::Infallible;
use std::fs::{self};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use warp::filters::multipart::FormData;
use warp::reject::Rejection;
use warp::reply::Reply;
use warp::{self, http::StatusCode, Buf};

const UPLOAD_DIR: &str = "filestore/";

pub async fn handle_file_upload(
    db: Arc<sled::Db>,
    mut form: FormData,
) -> Result<impl warp::Reply, Infallible> {
    //empty the current folder for new uploads since user can't have root hash for all files
    let _ = empty_folder(format!("./{}", UPLOAD_DIR));

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
        //clean file name for storage (remove all spaces and special characters)
        let clean_file_name = clean_file_name(&filename);
        let save_path = PathBuf::from(format!("./{}{}", UPLOAD_DIR, clean_file_name));
        if (tokio::fs::write(save_path, data).await).is_err() {
            return Ok(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    //clear old db entries before adding new ones
    let _ = clear_db(db.clone());
    //build merkle tree for the entire upload folder
    let file_hash_map = get_file_list();
    FastMerkleTree::build_merkle_tree(db, file_hash_map);
    Ok(StatusCode::OK)
}

pub async fn handle_file_download(
    db: Arc<sled::Db>,
    filename: String,
) -> Result<impl Reply, Rejection> {
    use tokio_util::io::ReaderStream;
    let filepath = PathBuf::from(format!("./filestore/{}", filename));
    if filepath.exists() {
        let mut merkle_proof: Vec<(Vec<u8>, bool)> = Vec::new();

        // get merkle proof from db
        if let Some(proof) =
            FastMerkleTree::get_merkle_proof_from_db(db, format!("./filestore/{}", filename))
        {
            merkle_proof = proof;
        };
        let file = tokio::fs::File::open(filepath).await.unwrap();

        let stream = ReaderStream::new(file);
        let response = warp::http::response::Builder::new()
            .header(
                "Content-Disposition",
                format!("attachment; filename=\"{}\"", filename),
            )
            .header("X-File-Hash", format!("{:?}", merkle_proof))
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

fn get_file_list() -> Vec<String> {
    let mut file_list: Vec<String> = Vec::new(); //replace with more concrete type
    let dir_path = format!("./{}", UPLOAD_DIR);
    let dir = Path::new(&dir_path);
    let entries = fs::read_dir(dir).unwrap();
    for entry in entries {
        let path = entry.unwrap().path();
        file_list.push(path.display().to_string())
    }
    file_list
}

fn clear_db(db: Arc<sled::Db>) -> Result<()> {
    for result in db.iter() {
        let (key, _) = result?;
        db.remove(key)?;
    }
    //   db.flush()?;
    Ok(())
}

fn empty_folder(path: String) -> Result<()> {
    let folder_path = Path::new(&path);
    for entry in fs::read_dir(folder_path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            fs::remove_dir_all(&path)?;
        } else {
            fs::remove_file(&path)?
        }
    }
    Ok(())
}

fn clean_file_name(file_name: &str) -> String {
    let invalid_chars = Regex::new(r"[^\w\.\-]").unwrap();
    let trimmed = file_name.trim();
    let clean_file_name = invalid_chars.replace_all(trimmed, "_");
    clean_file_name.to_string()
}
