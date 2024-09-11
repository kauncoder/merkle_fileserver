use crate::merkletree::tree::{FastMerkleTree, OFFSET_ONE, OFFSET_TWO};
use blake3::Hash;
use std::{fs::remove_file, path::PathBuf};
use warp::filters::multipart::FormData;
use warp::reject::Rejection;

use futures::TryStreamExt;
use std::convert::Infallible;
use std::fs::File;
use std::io::Write;
use warp::{self, Buf};

fn verify_proof(file_name: String, proof: Vec<(Vec<u8>, bool)>, root_hash: Vec<u8>) -> bool {
    //convert proof to vector of hashes
    let filepath = PathBuf::from(&file_name);
    if !filepath.exists() {
        return false;
    }
    let bytes = std::fs::read(filepath.clone()).unwrap();
    let mut hash_value = blake3::Hasher::new();
    hash_value.update(&OFFSET_ONE);
    hash_value.update(&bytes);
    let hash_value = hash_value.finalize();
    let mut current_hash = *hash_value.as_bytes();

    for (sibling_hash, is_left) in proof {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&OFFSET_TWO);
        if is_left {
            hasher.update(&sibling_hash);
            hasher.update(&current_hash);
        } else {
            hasher.update(&current_hash);
            hasher.update(&sibling_hash);
        }
        current_hash = *hasher.finalize().as_bytes();
    }
    let result = current_hash.to_vec() == root_hash;
    //also delete the temp file
    let _ = remove_file(file_name);
    result
}

pub async fn handle_file_hash(mut form: FormData) -> Result<impl warp::Reply, Rejection> {
    let mut file_hash_list: Vec<Hash> = Vec::new();
    while let Ok(Some(part)) = form.try_next().await {
        if part.name() == "file" {
            // Stream the uploaded file and calculate its hash
            let mut hasher = blake3::Hasher::new();
            hasher.update(&OFFSET_ONE);
            let mut stream = part.stream();

            while let Ok(Some(chunk)) = stream.try_next().await {
                hasher.update(chunk.chunk());
            }

            // Calculate the final hash and convert it to a hexadecimal string
            let hash = hasher.finalize();
            file_hash_list.push(hash);
        }
    }
    //calculate the root hash
    let root_hash = format!(
        "{:?}",
        FastMerkleTree::get_root_hash_from_leaves(file_hash_list)
            .value
            .as_bytes()
            .to_vec()
    );

    let response = warp::http::response::Builder::new()
        .header("Content-Type", "text/plain")
        .body(root_hash)
        .unwrap();

    Ok(response)
}

pub async fn handle_verify(
    mut form: warp::multipart::FormData,
) -> Result<impl warp::Reply, Infallible> {
    let mut root_hash = String::new();
    let mut merkle_proof = String::new();
    let mut file_name = String::new();

    // Iterate through the form fields
    while let Ok(Some(part)) = form.try_next().await {
        match part.name() {
            "file" => {
                // Handle the file upload
                let file_path = format!("./{}", part.filename().unwrap_or("uploaded_file"));
                let mut file = File::create(&file_path).unwrap();

                let mut stream = part.stream();
                while let Ok(Some(chunk)) = stream.try_next().await {
                    file.write_all(chunk.chunk()).unwrap();
                }

                file_name = file_path;
            }
            "value1" => {
                // Get the first string value
                let mut data = Vec::new();
                let mut stream = part.stream();
                while let Ok(Some(chunk)) = stream.try_next().await {
                    data.extend(chunk.chunk());
                }
                root_hash = String::from_utf8(data).unwrap();
            }
            "value2" => {
                // Get the second string value
                let mut data = Vec::new();
                let mut stream = part.stream();
                while let Ok(Some(chunk)) = stream.try_next().await {
                    data.extend(chunk.chunk());
                }
                merkle_proof = String::from_utf8(data).unwrap();
            }
            _ => {}
        }
    }

    //format inputs into usable types for inner functions
    let hash: Vec<u8> = serde_json::from_str(&root_hash).unwrap();
    let formatted_string = merkle_proof
        .replace('(', "[") // Replace '(' with '['
        .replace(')', "]"); // Replace ')' with ']'

    let proof: Vec<(Vec<u8>, bool)> = serde_json::from_str(&formatted_string).unwrap();
    let res = verify_proof(file_name, proof, hash);
    let result = match res {
        true => "Verification Passed",
        false => "Verification Failed",
    };

    Ok(warp::http::response::Builder::new()
        .header("Content-Type", "text/plain")
        .body(result.to_string()))
}
