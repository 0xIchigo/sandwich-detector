use dotenv::dotenv;
use std::env;

use ::helius::types::Cluster;
use helius::error::Result;
use helius::Helius;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let api_key: String = env::var("HELIUS_API_KEY").expect("HELIUS_API_KEY not found");
    let cluster: Cluster = Cluster::MainnetBeta;

    let helius: Helius = Helius::new(&api_key, cluster).unwrap();

    println!("Successfully created a Helius client");
    Ok(())
}
