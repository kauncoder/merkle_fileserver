use anyhow::Result;
use blake3::{Hash, Hasher};
use std::sync::Arc;

pub struct FastMerkleTree(pub Vec<FastMerkleNode>);

#[derive(Clone)]
pub struct FastMerkleNode {
    pub value: Hash,
}

pub const ZERO: [u8; 32] = [0; 32];
pub const OFFSET_ONE: [u8; 4] = 1u32.to_le_bytes(); //for leaf nodes
pub const OFFSET_TWO: [u8; 4] = 2u32.to_le_bytes(); //for inner nodes

impl FastMerkleNode {
    pub fn default() -> Self {
        FastMerkleNode {
            value: Hash::from_bytes(ZERO),
        }
    }
}

fn get_file_hashes(file_list: Vec<String>) -> Vec<Hash> {
    //read files and return vec of file hashes
    let mut file_hash_list: Vec<Hash> = Vec::new();
    for file in file_list.clone() {
        let file_content = std::fs::read(file.clone()).unwrap();
        let mut hash = blake3::Hasher::new();
        hash.update(&OFFSET_ONE);
        hash.update(&file_content);
        let hash = hash.finalize();
        file_hash_list.push(hash);
    }
    file_hash_list
}

impl FastMerkleTree {
    // Build the Merkle tree as an array of hashes
    pub fn build_merkle_tree(db: Arc<sled::Db>, file_list: Vec<String>) {
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
            tree[leaf_start + i] = FastMerkleNode { value: leaf_hash };
        }
        for i in (0..leaf_start).rev() {
            let left_child = &tree[2 * i + 1];
            let right_child = &tree[2 * i + 2];

            let mut hasher = Hasher::new();
            hasher.update(&OFFSET_TWO); //for inner nodes
            hasher.update(left_child.value.as_bytes());
            hasher.update(right_child.value.as_bytes());
            let hash = hasher.finalize();
            tree[i] = FastMerkleNode { value: hash };
        }
        let _ = Self::store_merkle_tree(db, FastMerkleTree(tree));
    }

    #[allow(dead_code)]
    pub fn pretty_merkle_proof(merkle_proof: Vec<(Vec<u8>, bool)>) -> Vec<(String, bool)> {
        let mut pretty_proof: Vec<(String, bool)> = Vec::new();
        for (node, is_left) in merkle_proof {
            pretty_proof.push((
                node.iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<Vec<_>>()
                    .join(""),
                is_left,
            ));
        }
        pretty_proof
    }

    fn store_merkle_tree(db: Arc<sled::Db>, merkle_tree: FastMerkleTree) -> Result<()> {
        //stores tree in db
        let mut i: usize = 0;
        for node in merkle_tree.0 {
            db.insert(i.to_le_bytes(), node.value.as_bytes())?;
            i += 1;
        }
        let tree_size: usize = i; // merkle_tree.0.len().try_into().unwrap();
        let _ = db.insert(b"tree_size", &tree_size.to_le_bytes());
        Ok(())
    }

    fn store_file_list(db: Arc<sled::Db>, file_list: Vec<String>) -> Result<()> {
        let mut i: usize = 0;
        for filename in file_list {
            db.insert(filename.as_bytes(), &i.to_le_bytes())?;
            i += 1;
        }
        //also store number of files
        let num_of_files: usize = i; // merkle_tree.0.len().try_into().unwrap();
        let _ = db.insert(b"num_of_files", &num_of_files.to_le_bytes());
        Ok(())
    }

    pub fn get_merkle_proof_from_db(
        db: Arc<sled::Db>,
        filename: String,
    ) -> Option<Vec<(Vec<u8>, bool)>> {
        let mut tree_size: usize = 0;
        if let Some(value) = db.get(b"tree_size").unwrap() {
            tree_size = usize::from_le_bytes(value.as_ref().try_into().unwrap());
        }
        let mut file_index: usize = 0;
        if let Some(value) = db.get(filename.as_bytes()).unwrap() {
            file_index = usize::from_le_bytes(value.as_ref().try_into().unwrap());
        }
        let mut leaf_count: usize = 0;
        if let Some(value) = db.get(b"num_of_files").unwrap() {
            leaf_count = usize::from_le_bytes(value.as_ref().try_into().unwrap());
            if leaf_count % 2 != 0 {
                leaf_count += 1;
            }
        }
        let mut index = tree_size - leaf_count + file_index;
        let mut proof = Vec::new();

        while index > 0 {
            let sibling_index = if index % 2 == 0 { index - 1 } else { index + 1 };
            if let Some(value) = db.get(sibling_index.to_le_bytes()).unwrap() {
                let node = value.to_vec();
                let is_left = index % 2 == 0;
                proof.push((node, is_left));
                index = (index - 1) / 2
            }
        }
        Some(proof)
    }

    pub fn get_root_hash_from_leaves(leaves: Vec<Hash>) -> FastMerkleNode {
        let mut leaves = leaves;
        let leaf_count = leaves.len();
        //balance the tree
        if leaf_count % 2 != 0 {
            leaves.push(*leaves.last().unwrap());
        }
        let total_nodes = 2 * leaves.len() - 1;
        // add leaf nodes
        let mut tree: Vec<FastMerkleNode> = vec![FastMerkleNode::default(); total_nodes];
        let leaf_start = leaves.len() - 1;
        for (i, leaf_hash) in leaves.into_iter().enumerate() {
            tree[leaf_start + i] = FastMerkleNode { value: leaf_hash };
        }
        //populate internal nodes
        for i in (0..leaf_start).rev() {
            let left_child = &tree[2 * i + 1];
            let right_child = &tree[2 * i + 2];

            let mut hasher = blake3::Hasher::new();
            hasher.update(&OFFSET_TWO); //for inner nodes
            hasher.update(left_child.value.as_bytes());
            hasher.update(right_child.value.as_bytes());
            let hash = hasher.finalize();
            tree[i] = FastMerkleNode { value: hash };
        }

        tree[0].clone()
    }
}

#[cfg(test)] // This annotation ensures that the following code is only compiled when testing
mod tests {
    #[test]
    fn test_get_file_hashes() {
        use crate::fileserver::fs::get_file_list;
        use crate::merkletree::tree::get_file_hashes;
        const TEST_DIR: &str = "./testfiles";

        let test_file_hash_list_string = r#"[[246, 207, 76, 200, 105, 48, 50, 111, 16, 109, 151, 176, 250, 147, 234, 30, 41, 56, 90, 215, 237, 134, 65, 202, 250, 61, 222, 125, 47, 59, 26, 55], [180, 33, 151, 17, 193, 222, 26, 14, 239, 90, 125, 170, 80, 242, 72, 30, 250, 79, 115, 150, 216, 20, 229, 54, 97, 218, 159, 176, 158, 93, 95, 221], [155, 133, 41, 126, 76, 192, 241, 95, 144, 235, 9, 252, 31, 212, 120, 230, 179, 219, 57, 93, 96, 62, 247, 190, 215, 248, 118, 214, 140, 226, 159, 187], [238, 238, 225, 160, 96, 127, 5, 59, 26, 200, 76, 3, 232, 137, 19, 188, 135, 48, 153, 189, 233, 23, 12, 128, 54, 140, 152, 194, 132, 81, 229, 6], [117, 98, 196, 237, 200, 184, 238, 94, 54, 57, 169, 221, 163, 253, 208, 186, 55, 100, 42, 87, 249, 179, 235, 181, 180, 170, 255, 104, 250, 78, 3, 228], [91, 43, 237, 104, 177, 76, 57, 213, 1, 53, 249, 162, 208, 201, 21, 175, 245, 235, 210, 98, 138, 184, 84, 25, 181, 246, 141, 72, 51, 166, 9, 65], [122, 58, 70, 144, 73, 18, 223, 64, 204, 135, 28, 67, 28, 179, 154, 5, 126, 35, 139, 215, 111, 225, 109, 161, 129, 151, 203, 87, 79, 8, 216, 180], [177, 65, 85, 64, 58, 125, 38, 7, 114, 131, 199, 70, 170, 99, 130, 141, 218, 25, 26, 115, 239, 160, 179, 92, 39, 53, 3, 13, 92, 4, 194, 115]]"#;
        let file_hash_list: Vec<Vec<u8>> =
            serde_json::from_str(test_file_hash_list_string).unwrap();

        let file_list: Vec<String> = get_file_list(TEST_DIR);
        println!("file hash list {:?}",file_hash_list);
        let file_hashes = get_file_hashes(file_list)
            .iter()
            .map(|h| h.as_bytes().to_vec())
            .collect::<Vec<_>>();
        assert_eq!(file_hashes, file_hash_list)
    }
}
