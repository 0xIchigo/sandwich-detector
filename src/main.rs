use dotenv::dotenv;
use solana_sdk::signature::Signature;
use std::{collections::HashMap, env, str::FromStr};

use helius::error::Result;
use helius::types::Cluster;
use helius::Helius;

use hex;
use solana_client::rpc_config::{RpcBlockConfig, RpcTransactionConfig};
use solana_sdk::{message::VersionedMessage, transaction::VersionedTransaction};
use solana_transaction_status::{
    EncodedConfirmedTransactionWithStatusMeta, EncodedTransactionWithStatusMeta, TransactionDetails, UiConfirmedBlock,
    UiTransactionEncoding, UiTransactionStatusMeta,
};

use sandwich_detector::types::{get_instruction_map, TARGET_PROGRAM};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let api_key: String = env::var("HELIUS_API_KEY").expect("HELIUS_API_KEY not found");
    let cluster: Cluster = Cluster::MainnetBeta;

    let helius: Helius = Helius::new(&api_key, cluster).unwrap();
    println!("Successfully created a Helius client");

    let signature: Signature =
        Signature::from_str("3Z1fWYJmKKsvnYY5BmZxnx4hmDYaPzB8RfH6GXoBy5tjp1JMarGK6xPypCv3D7SW98E761p3mYaUhq1K5JANFWjN")
            .unwrap();

    let config: RpcTransactionConfig = RpcTransactionConfig {
        encoding: Some(UiTransactionEncoding::Base64),
        commitment: None,
        max_supported_transaction_version: Some(0),
    };

    let tx_with_meta: EncodedConfirmedTransactionWithStatusMeta = helius
        .connection()
        .get_transaction_with_config(&signature, config)
        .unwrap();

    // Check if the transaction has an AutoSwapIn instruction
    if let Some(instruction_name) = find_known_instruction(&tx_with_meta) {
        println!("This transaction contains a known instruction: {}", instruction_name)
    } else {
        println!("No known instructions found in this transaction")
    }

    Ok(())
}

// Checks if a given transaction contains a known instructions
fn find_known_instruction(tx_with_meta: &EncodedConfirmedTransactionWithStatusMeta) -> Option<&'static str> {
    let versioned_tx: VersionedTransaction = match tx_with_meta.transaction.transaction.decode() {
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
            let hex_data = hex::encode(&ix.data);

            for (discriminator, name) in &instruction_map {
                if hex_data.contains(discriminator) {
                    return Some(name);
                }
            }
        }
    }

    None
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
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
