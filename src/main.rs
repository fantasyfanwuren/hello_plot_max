use hello_plot_max::*;
use log::{debug, error, info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log4rs::init_file("./log4rs.yaml", Default::default()).unwrap();
    // test

    let user_set = get_user_set().await?;
    info!("{:?}", user_set);

    let s = ShowInfos::new(user_set).await?;
    s.show().await;

    Ok(())
}
