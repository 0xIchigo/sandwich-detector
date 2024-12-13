use dotenv::dotenv;
use std::env;

use helius::error::Result;
use helius::types::Cluster;
use helius::Helius;

use solana_client::rpc_config::RpcBlockConfig;
use solana_transaction_status::{
    EncodedTransactionWithStatusMeta, TransactionDetails, UiConfirmedBlock, UiTransactionEncoding,
    UiTransactionStatusMeta,
};

// use sandwich_detector::types::{ClassifiedTransaction, Pattern};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let api_key: String = env::var("HELIUS_API_KEY").expect("HELIUS_API_KEY not found");
    let cluster: Cluster = Cluster::MainnetBeta;

    let helius: Helius = Helius::new(&api_key, cluster).unwrap();
    println!("Successfully created a Helius client");

    // Example
    let recent_blocks: Vec<UiConfirmedBlock> = get_recent_blocks(&helius, 5).await?;

    println!("Analyzing {} blocks", recent_blocks.len());
    for (i, block) in recent_blocks.iter().enumerate() {
        println!("\nAnalyzing Block {}:", i + 1);
        analyze_non_vote_transactions(block)?;
    }

    Ok(())
}

async fn get_recent_blocks(helius: &Helius, num_blocks: u64) -> Result<Vec<UiConfirmedBlock>> {
    let current_slot: u64 = helius.connection().get_slot()?;
    let mut blocks: Vec<UiConfirmedBlock> = Vec::new();

    let config: RpcBlockConfig = RpcBlockConfig {
        commitment: None,
        max_supported_transaction_version: Some(0),
        transaction_details: Some(TransactionDetails::Full),
        rewards: Some(true),
        encoding: Some(UiTransactionEncoding::Json),
    };

    for slot in (current_slot.saturating_sub(num_blocks)..current_slot).rev() {
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

fn is_transaction_successful(meta: &UiTransactionStatusMeta) -> bool {
    match meta.err {
        None => true,
        Some(_) => false,
    }
}

#[allow(dead_code)]
async fn analyze_block_transactions(block: &UiConfirmedBlock) -> Result<()> {
    if let Some(transactions) = &block.transactions {
        // Only looking at the first tx to start
        if let Some(first_tx) = transactions.first() {
            println!("{:?}", first_tx)
        }
    } else {
        println!("No transactions found in this block");
    }

    Ok(())
}

fn analyze_non_vote_transactions(block: &UiConfirmedBlock) -> Result<()> {
    const TARGET_PROGRAM: &str = "vpeNALD89BZ4KxNUFjdLmFXBCwtyqBDQ85ouNoax38b";

    if let Some(transactions) = &block.transactions {
        let non_vote_txs: Vec<&EncodedTransactionWithStatusMeta> = transactions
            .iter()
            .filter(|tx| {
                if let Some(meta) = &tx.meta {
                    if !is_transaction_successful(meta) {
                        return false;
                    }

                    let logs: Option<Vec<String>> = meta.log_messages.clone().into();

                    if let Some(logs) = logs {
                        let is_vote: bool = logs
                            .iter()
                            .any(|log| log.contains("Vote111111111111111111111111111111111111111"));
                        let has_target: bool = logs.iter().any(|log| log.contains(TARGET_PROGRAM));

                        !is_vote && has_target
                    } else {
                        false
                    }
                } else {
                    // If no meta, treat as non-vote
                    true
                }
            })
            .collect();

        // Print details of each non-vote transaction
        for (i, tx) in non_vote_txs.iter().enumerate() {
            println!("\nTransaction {}", i + 1);
            if let Some(meta) = &tx.meta {
                let logs: Option<Vec<String>> = meta.log_messages.clone().into();
                if let Some(logs) = logs {
                    println!("Program Invocations:");
                    for log in logs {
                        println!("  {}", log);
                    }
                }
            }
        }
    } else {
        println!("No transactions found in this block");
    }

    Ok(())
}
