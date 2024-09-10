use blake3::{Hasher, Hash};
use std:: sync::Arc;
use anyhow::Result;

pub struct FastMerkleTree(pub Vec<FastMerkleNode>);

#[derive(Clone)]
pub struct FastMerkleNode{
    pub value: Hash
}

pub const ZERO :[u8;32] = [0; 32];
pub const OFFSET_ONE: [u8;4] =  1u32.to_le_bytes(); //for leaf nodes
pub const OFFSET_TWO: [u8;4] =  2u32.to_le_bytes(); //for inner nodes

impl FastMerkleNode{
   pub  fn default()->Self{
        FastMerkleNode{value: Hash::from_bytes(ZERO)}
    }
}

fn get_file_hashes(file_list: Vec<String>)->Vec<Hash>{
    //read files and return vec of file hashes
    let mut file_hash_list : Vec<Hash> = Vec::new();
    for file in file_list.clone(){
        let file_content = std::fs::read(file.clone()).unwrap();
        let mut hash = blake3::Hasher::new();
        hash.update(&OFFSET_ONE);
        hash.update(&file_content);
        let hash = hash.finalize();
        file_hash_list.push(hash);
    }
    file_hash_list
}


impl FastMerkleTree{

// Build the Merkle tree as an array of hashes
pub fn build_merkle_tree(db: Arc<sled::Db>,file_list: Vec<String>){

    let _ = Self::store_file_list(db.clone(), file_list.clone());


    let mut leaves = get_file_hashes(file_list);
    let leaf_count = leaves.len();
    //balance the tree
    if leaf_count % 2 != 0 {
        leaves.push(*leaves.last().unwrap());
    }
    let total_nodes = 2 * leaves.len() - 1;
    let mut tree: Vec<FastMerkleNode> = vec![FastMerkleNode::default(); total_nodes];
    let leaf_start = leaves.len() - 1;
    for (i, leaf_hash) in leaves.into_iter().enumerate() {
        tree[leaf_start + i] = FastMerkleNode{value: leaf_hash};
    }
    for i in (0..leaf_start).rev() {
        let left_child = &tree[2 * i + 1];
        let right_child = &tree[2 * i + 2];

        let mut hasher = Hasher::new();
        hasher.update(&OFFSET_TWO); //for inner nodes
        hasher.update(left_child.value.as_bytes());
        hasher.update(right_child.value.as_bytes());
        let hash =hasher.finalize();
        tree[i] = FastMerkleNode{value: hash};
    }
    let _ = Self::store_merkle_tree(db,FastMerkleTree(tree));
}


#[allow(dead_code)]
pub fn pretty_merkle_proof(merkle_proof: Vec<(Vec<u8>,bool)>)->Vec<(String,bool)>{
    let mut pretty_proof : Vec<(String,bool)> = Vec::new();
    for (node, is_left) in merkle_proof{
        pretty_proof.push((node.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(""),is_left));
    }
    pretty_proof
}

fn store_merkle_tree(db: Arc<sled::Db>, merkle_tree: FastMerkleTree)->Result<()>{
    //stores tree in db
    let mut i: usize =0;
    for node in merkle_tree.0{
        db.insert(i.to_le_bytes(), node.value.as_bytes())?;
        i += 1;
    }
    let tree_size: usize =i;// merkle_tree.0.len().try_into().unwrap();
    let _ = db.insert(b"tree_size",&tree_size.to_le_bytes()); 
    Ok(())
}

fn store_file_list(db: Arc<sled::Db>, file_list: Vec<String>)->Result<()>{
    let mut i: usize =0;
    for filename in file_list{
        db.insert(filename.as_bytes(),&i.to_le_bytes())?;
        i += 1;
    }
    //also store number of files
    let num_of_files: usize =i;// merkle_tree.0.len().try_into().unwrap();
    let _ = db.insert(b"num_of_files",&num_of_files.to_le_bytes()); 
    Ok(())

}



pub fn get_merkle_proof_from_db(db: Arc<sled::Db>, filename: String) -> Option<Vec<(Vec<u8>,bool)>> {

    let mut tree_size : usize =0;
    if let Some(value) = db.get(b"tree_size").unwrap() {
        tree_size = usize::from_le_bytes(value.as_ref().try_into().unwrap());
    }
    let mut file_index: usize =0;
    if let Some(value) = db.get(filename.as_bytes()).unwrap() {
        file_index = usize::from_le_bytes(value.as_ref().try_into().unwrap());
    }
    let mut leaf_count: usize =0;
    if let Some(value) = db.get(b"num_of_files").unwrap() {
        leaf_count = usize::from_le_bytes(value.as_ref().try_into().unwrap());
        if leaf_count % 2 != 0 {
            leaf_count+=1;
        }
    
    }
    let mut index = tree_size -  leaf_count + file_index;
    let mut proof = Vec::new();

    while index > 0 {
        let sibling_index = if index % 2 == 0 {
            index - 1 
        } else {
            index + 1
        };
        if let Some(value) = db.get(sibling_index.to_le_bytes()).unwrap()   {
        let node = value.to_vec();
        let is_left = index % 2 == 0;
        proof.push((node, is_left));
        index = (index - 1) / 2
        }
    }
    Some(proof)
}

}