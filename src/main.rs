use dotenv::dotenv;
use std::env;

use helius::error::Result;
use helius::types::Cluster;
use helius::Helius;

use solana_client::rpc_config::RpcBlockConfig;
use solana_transaction_status::UiConfirmedBlock;

// use sandwich_detector::types::{ClassifiedTransaction, Pattern};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let api_key: String = env::var("HELIUS_API_KEY").expect("HELIUS_API_KEY not found");
    let cluster: Cluster = Cluster::MainnetBeta;

    let helius: Helius = Helius::new(&api_key, cluster).unwrap();
    println!("Successfully created a Helius client");

    // Example - fetch last 5 blocks
    let recent_blocks: Vec<UiConfirmedBlock> = get_recent_blocks(&helius, 4).await?;

    for (i, block) in recent_blocks.iter().enumerate() {
        println!("\nBlock {}:", i + 1);
        println!("  Blockhash: {}", block.blockhash);
        println!("  Previous Blockhash: {}", block.previous_blockhash);
        println!("  Parent Slot: {}", block.parent_slot);
        println!(
            "  Number of Transactions: {}",
            block.transactions.as_ref().map_or(0, |txs| txs.len())
        );

        if let Some(time) = block.block_time {
            println!("  Block Time: {}", time);
        }

        if let Some(height) = block.block_height {
            println!("  Block Height: {}", height);
        }
    }
    Ok(())
}

async fn get_recent_blocks(helius: &Helius, num_blocks: u64) -> Result<Vec<UiConfirmedBlock>> {
    let current_slot: u64 = helius.connection().get_slot()?;
    let mut blocks: Vec<solana_transaction_status::UiConfirmedBlock> = Vec::new();

    let config: RpcBlockConfig = RpcBlockConfig {
        encoding: None,
        transaction_details: None,
        rewards: None,
        commitment: None,
        max_supported_transaction_version: Some(0),
    };

    for slot in (current_slot.saturating_sub(num_blocks)..=current_slot).rev() {
        match helius.connection().get_block_with_config(slot, config.clone()) {
            Ok(block) => {
                blocks.push(block);
            }
            Err(e) => {
                eprintln!("Failed to fetch block at slot {}: {}", slot, e);
                continue;
            }
        }
    }

    Ok(blocks)
}
