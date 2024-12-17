use dotenv::dotenv;
use std::{collections::HashMap, env};

use helius::error::Result;
use helius::types::Cluster;
use helius::Helius;

use hex::encode;
use solana_client::rpc_config::RpcBlockConfig;
use solana_sdk::{message::VersionedMessage, pubkey::Pubkey, transaction::VersionedTransaction};
use solana_transaction_status::{
    EncodedTransactionWithStatusMeta, TransactionDetails, UiConfirmedBlock, UiTransactionEncoding,
    UiTransactionStatusMeta,
};

use sandwich_detector::types::{
    get_instruction_map, ClassifiedTransaction, JITO_TIP_ADDRESSES, MIN_JITO_TIP, TARGET_PROGRAM,
};

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
fn find_known_instruction(
    tx_with_meta: &EncodedTransactionWithStatusMeta,
    slot: u64,
    block_time: Option<u64>,
) -> Vec<ClassifiedTransaction> {
    let versioned_tx: VersionedTransaction = match tx_with_meta.transaction.decode() {
        Some(tx) => tx,
        None => return vec![],
    };

    let instruction_map: HashMap<&str, &str> = get_instruction_map();

    let (account_keys, instructions) = match &versioned_tx.message {
        VersionedMessage::Legacy(msg) => (msg.account_keys.clone(), msg.instructions.clone()),
        VersionedMessage::V0(msg) => (msg.account_keys.clone(), msg.instructions.clone()),
    };

    let signature: String = if !versioned_tx.signatures.is_empty() {
        versioned_tx.signatures[0].to_string()
    } else {
        "".to_string()
    };

    let signer: String = {
        let num_signers = versioned_tx.message.header().num_required_signatures as usize;

        if num_signers > 0 && account_keys.len() >= num_signers {
            account_keys[0].to_string()
        } else {
            "".to_string()
        }
    };

    let target_program_idx: Option<usize> = account_keys.iter().position(|key| key.to_string() == TARGET_PROGRAM);
    let mut found_txs = Vec::new();

    for ix in &instructions {
        if ix.program_id_index as usize == target_program_idx.unwrap_or_default() {
            // Ensure the instruction data is at least 8 bytes so we can extract the discriminator
            if ix.data.len() < 8 {
                continue;
            }

            let discriminator_bytes: &[u8] = &ix.data[0..8];
            let hex_data: String = encode(discriminator_bytes);

            if let Some(name) = instruction_map.get(hex_data.as_str()) {
                // Setting to default values for now
                let mut sandwich_acc: String = "".to_string();
                let from_mint: String = "".to_string();
                let to_mint: String = "".to_string();
                let from_amount: u64 = 0;
                let to_amount: u64 = 0;

                match *name {
                    "CreateSandwichV2" => {
                        if ix.accounts.len() > 2 {
                            sandwich_acc = account_keys[ix.accounts[2] as usize].to_string();
                        }
                    }
                    "AutoSwapIn" | "AutoSwapOut" => {
                        if ix.accounts.len() > 6 {
                            sandwich_acc = account_keys[ix.accounts[6] as usize].to_string();
                        } else if ix.accounts.len() > 7 {
                            sandwich_acc = account_keys[ix.accounts[7] as usize].to_string();
                        }
                    }
                    // Decode the to/from mints here
                    // For now, we'll leave it empty
                    _ => {}
                }

                let classified_tx: ClassifiedTransaction = ClassifiedTransaction {
                    signature: signature.clone(),
                    signer: signer.clone(),
                    slot,
                    block_time,
                    instruction_type: name.to_string(),
                    sandwich_acc,
                    from_mint,
                    to_mint,
                    from_amount,
                    to_amount,
                };

                found_txs.push(classified_tx);
            }
        }
    }

    found_txs
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

// Checks if an address is a Jito tip address
#[allow(dead_code)]
fn is_jito_tip_address(addr: &str) -> bool {
    JITO_TIP_ADDRESSES.contains(&addr)
}

// Checks Jito tups by comparing pre- and post-balances
#[allow(dead_code)]
fn detect_jito_tip(account_keys: &[Pubkey], pre_balances: &[u64], post_balances: &[u64]) -> u64 {
    let mut total_tip: u64 = 0;

    for (i, key) in account_keys.iter().enumerate() {
        let diff: u64 = post_balances[i].saturating_sub(pre_balances[i]);

        if diff >= MIN_JITO_TIP && is_jito_tip_address(&key.to_string()) {
            total_tip += diff;
        }
    }

    total_tip
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

        let slot: u64 = block.block_height.unwrap_or(0);
        let block_time: Option<u64> = block.block_time.map(|x| x as u64);

        for (_i, tx) in non_vote_txs.iter().enumerate() {
            let classified_txs: Vec<ClassifiedTransaction> = find_known_instruction(tx, slot, block_time);
            for classified_tx in classified_txs {
                // println!(
                //     "Found known instruction: {} with sandwich_acc: {}",
                //     classified_tx.instruction_type, classified_tx.sandwich_acc
                // );
                println!("{}", serde_json::to_string_pretty(&classified_tx).unwrap());
                println!("Sus transaction: {}", serde_json::to_string_pretty(&tx).unwrap());
            }
        }
    } else {
        println!("No transactions found in this block");
    }

    Ok(())
}
