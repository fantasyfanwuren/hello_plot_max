use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct UserSet {
    pub source_dir_path: String,
    pub hdd_limit_rate: f32,
    pub final_dirs: Vec<FinalDir>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct FinalDir {
    pub path: String,
    pub size: f32,
}

pub async fn get_user_set() -> Result<UserSet, Box<dyn std::error::Error>> {
    // 读取"./userset.json"
    let use_set: UserSet = {
        // 读取"./userset.json"
        let set_str = std::fs::read_to_string("./userset.json")?;

        // 序列化
        serde_json::from_str(&set_str)?
    };
    Ok(use_set)
}
