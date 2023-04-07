use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct UserSet {
    pub source_dir_path: String,
    pub final_dirs: Vec<FinalDir>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct FinalDir {
    pub path: String,
    pub size: f32,
}
