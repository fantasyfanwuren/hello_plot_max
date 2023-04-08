use hello_plot_max::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log4rs::init_file("./log4rs.yaml", Default::default()).unwrap();
    std::fs::write("./log/requests.log", "")?;
    run().await?;
    Ok(())
}
