use hello_plot_max::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log4rs::init_file("./log4rs.yaml", Default::default()).unwrap();
    // test
    let user_set = std::fs::read_to_string("./userset.json")?;
    let use_set: UserSet = serde_json::from_str(&user_set)?;
    println!("{:?}", use_set);

    Ok(())
}
