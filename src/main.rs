use hello_plot_max::*;
use log::error;
use std::panic;

fn handle_panic(info: &panic::PanicInfo<'_>) {
    if let Some(location) = info.location() {
        error!(
            "Panic occurred in file '{}' at line {}",
            location.file(),
            location.line(),
        );
    } else {
        error!("Panic occurred with no location information.");
    }

    if let Some(payload) = info.payload().downcast_ref::<String>() {
        error!("Panic message: {}", payload);
    } else {
        error!("Panic message: <no message>");
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log4rs::init_file("./log4rs.yaml", Default::default()).unwrap();
    std::fs::write("./log/requests.log", "")?;
    panic::set_hook(Box::new(|info| {
        handle_panic(info);
    }));
    run().await.unwrap();
    Ok(())
}
