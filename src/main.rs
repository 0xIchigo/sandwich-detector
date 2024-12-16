use dotenv::dotenv;
use std::{collections::HashMap, env};

use helius::error::Result;
use helius::types::Cluster;
use helius::Helius;

use hex::encode;
use solana_client::rpc_config::RpcBlockConfig;
use solana_sdk::{message::VersionedMessage, transaction::VersionedTransaction};
use solana_transaction_status::{
    EncodedTransactionWithStatusMeta, TransactionDetails, UiConfirmedBlock, UiTransactionEncoding,
    UiTransactionStatusMeta,
};

use sandwich_detector::types::{get_instruction_map, TARGET_PROGRAM};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let api_key: String = env::var("HELIUS_API_KEY").expect("HELIUS_API_KEY not found");
    let cluster: Cluster = Cluster::MainnetBeta;

    let helius: Helius = Helius::new(&api_key, cluster).unwrap();
    println!("Successfully created a Helius client");

    let recent_blocks: Vec<UiConfirmedBlock> = get_recent_blocks(&helius, 5).await?;
    println!("Analyzing {} blocks", recent_blocks.len());

    for (i, block) in recent_blocks.iter().enumerate() {
        println!("\nAnalyzing Block {}:", i + 1);
        analyze_non_vote_transactions(block)?;
    }

    Ok(())
}

// Checks if a given transaction contains a known instructions
fn find_known_instruction(tx_with_meta: &EncodedTransactionWithStatusMeta) -> Option<&'static str> {
    let versioned_tx: VersionedTransaction = match tx_with_meta.transaction.decode() {
        Some(tx) => tx,
        None => return None,
    };

    let instruction_map: HashMap<&str, &str> = get_instruction_map();

    let (account_keys, instructions) = match &versioned_tx.message {
        VersionedMessage::Legacy(msg) => (msg.account_keys.clone(), msg.instructions.clone()),
        VersionedMessage::V0(msg) => (msg.account_keys.clone(), msg.instructions.clone()),
    };

    let target_program_idx: Option<usize> = account_keys.iter().position(|key| key.to_string() == TARGET_PROGRAM);

    for ix in &instructions {
        if ix.program_id_index as usize == target_program_idx.unwrap_or_default() {
            // Ensure the instruction data is at least 8 bytes so we can extract the discriminator
            if ix.data.len() < 8 {
                continue;
            }

            let discriminator_bytes: &[u8] = &ix.data[0..8];
            let hex_data: String = encode(discriminator_bytes);

            if let Some(name) = instruction_map.get(hex_data.as_str()) {
                return Some(name);
            }
        }
    }

    None
}

async fn get_recent_blocks(helius: &Helius, num_blocks: u64) -> Result<Vec<UiConfirmedBlock>> {
    let current_slot: u64 = helius.connection().get_slot()?;
    let mut blocks: Vec<UiConfirmedBlock> = Vec::new();

    let config: RpcBlockConfig = RpcBlockConfig {
        commitment: None,
        max_supported_transaction_version: Some(0),
        transaction_details: Some(TransactionDetails::Full),
        rewards: Some(true),
        encoding: Some(UiTransactionEncoding::Base64),
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

        for (_i, tx) in non_vote_txs.iter().enumerate() {
            // Check if this transaction contains a known transaction
            if let Some(instruction_name) = find_known_instruction(tx) {
                println!("Found known instruction: {}", instruction_name);
            } else {
                println!("No known instructions found in this transaction");
            }
        }
    } else {
        println!("No transactions found in this block");
    }

    Ok(())
}
