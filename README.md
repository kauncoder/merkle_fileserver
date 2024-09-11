# Merkle File Server

A simple fileserver to upload files into and receive a merkle proof of the file's correctness on download

## 1. Features

The code is divided into two parts: client and fileserver.
The client provides an interface to generate the roothash for a group of files.
Later it allows to verify the integrity of a particular file that was downloaded back from the fileserver.

The fileserver provides an interface to upload, view and download files
On file download it generates a merkle proof for the file.

The merkletree code provides implementation of basic merkle tree functionality

The merkle tree is maintained in a `sled` database.

## 2. Pre-requisites

This only requires `cargo` to work. You can install it from [here](https://www.rust-lang.org/learn/get-started). Some libraries have dependencies like `gcc` so install them as well. In ubuntu this is done using: 
```sudo apt update
sudo apt install build-essential
sudo apt install gcc
```

## 3. Usage


### 3.1. Running server

From CLI run ```cargo run server``` to start file-server. Then access the functions from browser at ```localhost:8080```

1. ```localhost:8080/upload```: to select and upload files
2. ```localhost:8080/list```: to view uploaded files. clicking a file takes to its download page.
3. ```localhost:8080/download/<filename>```: to download the file and get its merkle proof


### 3.2. Running client

From CLI run ```cargo run client``` to start client. Then access the functions from browser at ```localhost:8081```

1. ```localhost:8081/hash```: to select files to get their root hash
2. ```localhost:8081/verify```: to check the integrity of a selected file 

## 4. Process

1. Get the root hash for files to be uploaded from `localhost:8081/hash` path. copy the hash value including the square braces like `[....]` and store it somewhere. this is the `root_hash`
2. Upload the files to the server from `localhost:8080/upload`
3. View list of file at `localhost:8080/list`. to download click on any file link which will redirect to download page.
4. Download the file, its merkle proof will be displayed as a vector of tuples `[([..],..)]`. Copy this entire expression including the square braces. this is the `merkle_proof`
5. To verify, go to the client verify page at `localhost:8081/verify` and select the downloaded file and paste the `root_hash` and `merkle_proof` values from previous steps.

## 5. Caveats

Each time user uploads files to the fileserver, the server deletes the previous files and merkle tree. This is because the user can not regenerate their hash for the set of old+new files. A possible solution to this can be achieved with zero-knowledge proofs. The fileserver will now create a new merkle tree with all files old+new and return not only the root hash of the merkle tree but also a proof of correct computation.

## 6. To Do
[ ] Add tls support 
[ ] Add persistent login
[ ] Multi-user support
[ ] Deployment in cloud (with Certificate/KMS/Oauth support)


