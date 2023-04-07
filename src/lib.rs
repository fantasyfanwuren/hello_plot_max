pub mod show;
pub mod userset;
use log::{debug, error, info, warn};

pub use show::*;
pub use userset::*;

pub async fn get_plot_size(path: &str) -> Result<f32, Box<dyn std::error::Error>> {
    let metadata = std::fs::metadata(path)?;
    let file_size = metadata.len() as f32 / 1024.0 / 1024.0 / 1024.0;
    Ok(file_size)
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
