use crate::{config::Config, jsonrpc::server::run_server};

pub mod jsonrpc;
pub mod config;

#[tokio::main]
async fn main() {
    let res = run().await;
    match res {
        Err(err) => log::error!("{:?}", err),
        Ok(_) => log::info!("Done"),
    }
}

async fn run() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    pretty_env_logger::init();
    log::info!("Starting proxy...");

    let f = std::fs::File::open("config.yaml")?;
    let config: Config = serde_yaml::from_reader(f)?;

    run_server(config).await?;

    Ok(())
}
