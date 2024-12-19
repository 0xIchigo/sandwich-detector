use dotenv::dotenv;
use lazy_static::lazy_static;
use std::{
    collections::{HashMap, HashSet},
    env,
    str::FromStr,
    sync::Mutex,
};

use helius::error::Result;
use helius::types::Cluster;
use helius::Helius;

use hex::encode;
use solana_client::rpc_config::RpcBlockConfig;
use solana_sdk::{
    instruction::CompiledInstruction, message::VersionedMessage, pubkey::Pubkey, transaction::VersionedTransaction,
};
use solana_transaction_status::{
    EncodedTransactionWithStatusMeta, TransactionDetails, UiConfirmedBlock, UiTransactionEncoding,
    UiTransactionStatusMeta, UiTransactionTokenBalance,
};

use sandwich_detector::types::{
    get_instruction_map, ClassifiedTransaction, Pattern, PatternTracker, SwapInfo, JITO_TIP_ADDRESSES, MIN_JITO_TIP,
    TARGET_PROGRAM, WSOL_MINT,
};

lazy_static! {
    static ref DECIMALS_CACHE: Mutex<HashMap<String, u8>> = Mutex::new(HashMap::new());
}

// EXAMPLE CALL FOR AN IDENTIFIED SANDWICH
// let target_block: u64 = 308362517;

// let block = get_block_by_slot(&helius, target_block).unwrap();

// Fetch the specific block
// if let Some(block) = block {
//     println!("\nAnalyzing Block {}:", target_block);
//     analyze_non_vote_transactions(&helius, &block).await?; // Note the .await here
// } else {
//     println!("Block {} not found or failed to fetch.", target_block);
// }

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
        analyze_non_vote_transactions(&helius, block).await?;
    }

    Ok(())
}

pub async fn get_token_decimals(helius: &Helius, mint_address: &str) -> Result<u8> {
    // Check cache first
    if let Some(decimals) = DECIMALS_CACHE.lock().unwrap().get(mint_address) {
        return Ok(*decimals);
    }

    let mint_pubkey: Pubkey = Pubkey::from_str(mint_address).unwrap();
    let account_data: Vec<u8> = helius.connection().get_account_data(&mint_pubkey)?;

    // Token mint data has decimals at offset 44
    let decimals = account_data[44];

    // Cache the result
    DECIMALS_CACHE
        .lock()
        .unwrap()
        .insert(mint_address.to_string(), decimals);

    Ok(decimals)
}

#[allow(dead_code)]
fn get_block_by_slot(helius: &Helius, slot: u64) -> Result<Option<UiConfirmedBlock>> {
    let config: RpcBlockConfig = RpcBlockConfig {
        commitment: None,
        max_supported_transaction_version: Some(0),
        transaction_details: Some(TransactionDetails::Full),
        rewards: Some(true),
        encoding: Some(UiTransactionEncoding::Base64),
    };

    match helius.connection().get_block_with_config(slot, config) {
        Ok(block) => Ok(Some(block)),
        Err(e) => {
            eprintln!("Failed to fetch block at slot {}: {}", slot, e);
            Ok(None)
        }
    }
}

// Checks if a given transaction contains a known instructions
fn find_known_instruction(
    tx_with_meta: &EncodedTransactionWithStatusMeta,
    block_height: u64,
    block_time: Option<u64>,
) -> Vec<ClassifiedTransaction> {
    let versioned_tx: VersionedTransaction = match tx_with_meta.transaction.decode() {
        Some(tx) => tx,
        None => return vec![],
    };

    let instruction_map: HashMap<&str, &str> = get_instruction_map();
    let mut found_txs: Vec<ClassifiedTransaction> = Vec::new();
    let mut processed_types: HashSet<String> = HashSet::new();

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
    let signer_pubkey: Pubkey = match Pubkey::from_str(&signer) {
        Ok(pk) => pk,
        Err(_) => return vec![], // Invalid signer public key, but this shouldn't happen
    };
    let signer_index: usize = account_keys
        .iter()
        .position(|key| key == &signer_pubkey)
        .unwrap_or(usize::MAX);

    let target_program_idx: Option<usize> = account_keys.iter().position(|key| key.to_string() == TARGET_PROGRAM);

    let pre_token_balances: &[UiTransactionTokenBalance] = match &tx_with_meta.meta {
        Some(meta) => meta.pre_token_balances.as_ref().map(|v| v.as_slice()).unwrap_or(&[]),
        None => &[],
    };

    let post_token_balances: &[UiTransactionTokenBalance] = match &tx_with_meta.meta {
        Some(meta) => meta.post_token_balances.as_ref().map(|v| v.as_slice()).unwrap_or(&[]),
        None => &[],
    };

    let jito_tip_amount: u64 = match &tx_with_meta.meta {
        Some(meta) => detect_jito_tip(&account_keys, &meta.pre_balances, &meta.post_balances),
        None => 0,
    };

    let lamport_change: i64 = if signer_index < tx_with_meta.meta.as_ref().map_or(0, |m| m.pre_balances.len())
        && signer_index < tx_with_meta.meta.as_ref().map_or(0, |m| m.post_balances.len())
    {
        (tx_with_meta.meta.as_ref().unwrap().post_balances[signer_index] as i64)
            - (tx_with_meta.meta.as_ref().unwrap().pre_balances[signer_index] as i64)
    } else {
        0
    };

    for ix in &instructions {
        if ix.program_id_index as usize == target_program_idx.unwrap_or_default() {
            // Ensure the instruction data is at least 8 bytes so we can extract the discriminator
            if ix.data.len() < 8 {
                continue;
            }

            let discriminator_bytes: &[u8] = &ix.data[0..8];
            let hex_data: String = encode(discriminator_bytes);

            // Check if we've already processed this instruction type
            if processed_types.contains(&hex_data) {
                continue;
            }

            // Check if the discriminator matches any known instruction
            if let Some(name) = instruction_map.get(hex_data.as_str()) {
                processed_types.insert(hex_data);

                let mut sandwich_acc: String = String::new();

                match *name {
                    "CreateSandwichV2" => {
                        if ix.accounts.len() > 2 {
                            sandwich_acc = account_keys[ix.accounts[2] as usize].to_string();
                        }
                    }
                    "AutoSwapIn" | "AutoSwapOut" => {
                        let sandwich_acc_indices: [usize; 2] = [6, 7];

                        for &idx in &sandwich_acc_indices {
                            if idx < ix.accounts.len() {
                                let account_idx: usize = ix.accounts[idx] as usize;

                                if account_idx < account_keys.len() {
                                    // Additional check for the actual program account pattern
                                    let account: &Pubkey = &account_keys[account_idx];
                                    sandwich_acc = account.to_string();
                                    break; // Take the first valid match
                                }
                            }
                        }
                    }
                    _ => {}
                }

                let classified_tx: ClassifiedTransaction = if let Some(swap_info) =
                    find_token_accounts(ix.clone(), &account_keys, pre_token_balances, post_token_balances, name)
                {
                    ClassifiedTransaction {
                        signature: signature.clone(),
                        signer: signer.clone(),
                        block_height,
                        block_time,
                        instruction_type: name.to_string(),
                        sandwich_acc,
                        swapper: swap_info.swapper,
                        from_mint: swap_info.from_mint,
                        to_mint: swap_info.to_mint,
                        from_amount: swap_info.from_amount,
                        to_amount: swap_info.to_amount,
                        jito_tip_amount,
                        wsol_change: swap_info.wsol_change,
                        lamport_change,
                        decimals: swap_info.decimals,
                    }
                } else {
                    ClassifiedTransaction {
                        signature: signature.clone(),
                        signer: signer.clone(),
                        block_height,
                        block_time,
                        instruction_type: name.to_string(),
                        sandwich_acc,
                        swapper: String::new(),
                        from_mint: String::new(),
                        to_mint: String::new(),
                        from_amount: 0,
                        to_amount: 0,
                        jito_tip_amount,
                        wsol_change: None,
                        lamport_change,
                        decimals: 9,
                    }
                };

                found_txs.push(classified_tx);
            }
        }
    }

    found_txs
}

fn find_token_accounts(
    ix: CompiledInstruction,
    account_keys: &[Pubkey],
    pre_token_balances: &[UiTransactionTokenBalance],
    post_token_balances: &[UiTransactionTokenBalance],
    instruction_type: &str,
) -> Option<SwapInfo> {
    let mut swap_info: SwapInfo = SwapInfo::new();

    // Get the indices of accounts involved in the instruction
    let relevant_accounts: HashSet<usize> = ix
        .accounts
        .iter()
        .filter_map(|&idx| {
            let account_idx = idx as usize;
            if account_idx < account_keys.len() {
                Some(account_idx)
            } else {
                None
            }
        })
        .collect();

    // Create maps for pre and post balances
    let pre_map: HashMap<usize, &UiTransactionTokenBalance> = pre_token_balances
        .iter()
        .filter(|b| relevant_accounts.contains(&(b.account_index as usize)))
        .map(|b| (b.account_index as usize, b))
        .collect();

    let post_map: HashMap<usize, &UiTransactionTokenBalance> = post_token_balances
        .iter()
        .filter(|b| relevant_accounts.contains(&(b.account_index as usize)))
        .map(|b| (b.account_index as usize, b))
        .collect();

    // Track changes for each mint
    let mut other_mint_changes: HashMap<String, Vec<(f64, usize)>> = HashMap::new();
    let mut primary_mint = String::new();
    let mut max_abs_change = 0.0;
    let mut wsol_change: Option<f64> = None;

    // Identify the primary token being swapped (the one with the largest absolute change)
    for (idx, pre_balance) in pre_map.iter() {
        if let Some(post_balance) = post_map.get(idx) {
            // Convert lamports to SOL by dividing by 1e9
            let pre_amount = pre_balance.ui_token_amount.amount.parse::<f64>().unwrap_or(0.0) / 1e9;
            let post_amount = post_balance.ui_token_amount.amount.parse::<f64>().unwrap_or(0.0) / 1e9;
            let change = post_amount - pre_amount;

            if change.abs() > 0.0 && pre_balance.mint != WSOL_MINT {
                if change.abs() > max_abs_change {
                    max_abs_change = change.abs();
                    primary_mint = pre_balance.mint.clone();
                }
                other_mint_changes
                    .entry(pre_balance.mint.clone())
                    .or_default()
                    .push((change, *idx));
            }
        }
    }

    // Look for wSOL changes associated with the primary token swap
    if !primary_mint.is_empty() {
        let mut primary_accounts: HashSet<String> = HashSet::new();

        // Collect owners of accounts involved in the primary token swap
        if let Some(changes) = other_mint_changes.get(&primary_mint) {
            for &(_, idx) in changes {
                if let Some(balance) = pre_map.get(&idx) {
                    if let Some(owner) = &balance.owner.as_ref().map(|s| s.as_str()) {
                        primary_accounts.insert(owner.to_string());
                    }
                }
            }
        }

        // Identify wSOL changes only for the primary accounts
        for (idx, pre_balance) in pre_map.iter() {
            if let Some(post_balance) = post_map.get(idx) {
                if pre_balance.mint == WSOL_MINT {
                    if let Some(owner) = &pre_balance.owner.as_ref().map(|s| s.as_str()) {
                        if primary_accounts.contains(*owner) {
                            // Convert lamports to SOL
                            let pre_amount = pre_balance.ui_token_amount.amount.parse::<f64>().unwrap_or(0.0) / 1e9;
                            let post_amount = post_balance.ui_token_amount.amount.parse::<f64>().unwrap_or(0.0) / 1e9;

                            // **Calculate wSOL Change Based on Instruction Type**
                            // For "AutoSwapIn", wsol_change = post - pre (increase)
                            // For "AutoSwapOut", wsol_change = pre - post (decrease)
                            wsol_change = match instruction_type {
                                "AutoSwapIn" => Some(post_amount - pre_amount),
                                "AutoSwapOut" => Some(pre_amount - post_amount),
                                _ => None,
                            };

                            break; // Assume only one relevant wSOL change per instruction
                        }
                    }
                }
            }
        }
    }

    // Process the Primary token canges
    if let Some(token_changes) = other_mint_changes.get(&primary_mint) {
        let (decrease, increase): (Vec<_>, Vec<_>) = token_changes.iter().partition(|&&(change, _)| change < 0.0);

        if let (Some(&(dec_change, dec_idx)), Some(&(inc_change, _))) = (decrease.first(), increase.first()) {
            let decimals: u8 = 9; // Temp set - will get overwritten by RPC call later

            let decrease_amount: u64 = (dec_change.abs() * 10f64.powi(decimals as i32)) as u64;
            let increase_amount: u64 = (inc_change * 10f64.powi(decimals as i32)) as u64;

            swap_info.from_mint = primary_mint.clone();
            swap_info.from_amount = decrease_amount;
            swap_info.to_mint = primary_mint.clone();
            swap_info.to_amount = increase_amount;
            swap_info.wsol_change = wsol_change;
            swap_info.decimals = decimals;

            // Set swapper from the account with the decrease
            if let Some(pre_balance) = pre_map.get(&dec_idx) {
                if let Some(owner) = &pre_balance.owner.as_ref().map(|s| s.as_str()) {
                    swap_info.swapper = owner.to_string();
                }
            }

            // Filter out interactions with the holding account
            if swap_info.swapper == "DKLvbSugkGMf4PBMakfHW9BdvcYj7Y7FRbsiL6v5DRy2" {
                println!("Filtered out swap involving holding account: {}", swap_info.swapper);
                return None;
            }

            return Some(swap_info);
        }
    }

    None
}

// Fetches num_blocks recent blocks
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

// Checks whether a given transaction was successful
fn is_transaction_successful(meta: &UiTransactionStatusMeta) -> bool {
    meta.err.is_none()
}

// Checks if an address is a Jito tip address
fn is_jito_tip_address(addr: &str) -> bool {
    JITO_TIP_ADDRESSES.contains(&addr)
}

// Checks Jito tups by comparing pre- and post-balances
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

// Checks non-vote transactions in a block for potential sandwich attacks
pub async fn analyze_non_vote_transactions(helius: &Helius, block: &UiConfirmedBlock) -> Result<()> {
    if let Some(transactions) = &block.transactions {
        let mut pattern_tracker: PatternTracker = PatternTracker::new();

        // Filter for non-vote transactions
        let non_vote_txs: Vec<&EncodedTransactionWithStatusMeta> = transactions
            .iter()
            .filter(|tx| {
                if let Some(meta) = &tx.meta {
                    if !is_transaction_successful(meta) {
                        return false;
                    }

                    let logs: Option<Vec<String>> = meta.log_messages.clone().into();
                    if let Some(logs) = logs {
                        let is_vote = logs
                            .iter()
                            .any(|log| log.contains("Vote111111111111111111111111111111111111111"));
                        let has_target = logs.iter().any(|log| log.contains(TARGET_PROGRAM));
                        !is_vote && has_target
                    } else {
                        false
                    }
                } else {
                    false
                }
            })
            .collect();

        let block_height: u64 = block.block_height.unwrap_or(0);
        let block_time: Option<u64> = block.block_time.map(|x| x as u64);

        for tx in non_vote_txs {
            let mut classified_txs: Vec<ClassifiedTransaction> = find_known_instruction(tx, block_height, block_time);

            for classified_tx in &mut classified_txs {
                if !classified_tx.from_mint.is_empty() {
                    match get_token_decimals(helius, &classified_tx.from_mint).await {
                        Ok(decimals) => {
                            classified_tx.decimals = decimals;
                        }
                        Err(e) => {
                            eprintln!("Failed to fetch decimals for token {}: {}", classified_tx.from_mint, e);
                        }
                    }
                }
            }

            for classified_tx in classified_txs {
                pattern_tracker.process_transaction(classified_tx);
            }
        }

        let completed_patterns: &[Pattern] = pattern_tracker.get_completed_patterns();

        if !completed_patterns.is_empty() {
            println!(
                "\n=== Found {} sandwich patterns at block height {} ===\n",
                completed_patterns.len(),
                block_height
            );

            for pattern in completed_patterns {
                println!("{}", pattern.to_summary());
                println!("---");
            }
        }
    }

    Ok(())
}
