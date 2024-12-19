use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::HashMap;

pub const MIN_JITO_TIP: u64 = 1000;
pub const TARGET_PROGRAM: &str = "vpeNALD89BZ4KxNUFjdLmFXBCwtyqBDQ85ouNoax38b";
pub const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";

pub const JITO_TIP_ADDRESSES: [&str; 8] = [
    "96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5",
    "HFqU5x63VTqvQss8hp11i4wVV8bD44PvwucfZ2bU7gRe",
    "Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY",
    "ADaUMid9yfUytqMBgopwjb2DTLSokTSzL1zt6iGPaS49",
    "DfXygSm4jCyNCybVYYK6DwvWqjKee8pbDmJGcLWNDXjh",
    "ADuUkR4vqLUMWXxW9gh6D6L8pMSawimctcNZ5pGwDcEt",
    "DttWaMuVvTiduZRnguLF7jNxTgiMBZ1hyAumKUiL2KRL",
    "3AVi9Tg9Uo68tJfuvoKvqKNWKkC5wPdSSdeBnizKZ6jT",
];

// Discriminators are hex strings that identify each instruction
// The key is the hex discriminator, the value is the instruction name
pub fn get_instruction_map() -> HashMap<&'static str, &'static str> {
    let mut m: HashMap<&str, &str> = HashMap::new();

    // Currently, we're only interested in the following instructions
    // However, others do exist
    m.insert("b3ecc1a00df8fe9a", "CreateSandwichV2");
    m.insert("5bb527f9eccb5e90", "AutoSwapIn");
    m.insert("b024faebda2bde25", "AutoSwapOut");
    // m.insert("14d812f9d70bd653", "Cashout");
    // m.insert("ea200c477e05dba0", "Exit");
    // m.insert("7edb0b2a6825518b", "ExitPrice");
    // m.insert("55e5a4f78f5c0591", "ExitInactivity");
    // m.insert("b404ba74bbf3e278", "MigrateTokenData");

    m
}

#[derive(Serialize)]
pub struct ClassifiedTransaction {
    pub signature: String,
    pub signer: String,
    pub block_height: u64,
    pub block_time: Option<u64>,
    pub instruction_type: String,
    pub sandwich_acc: String,
    pub swapper: String,
    pub from_mint: String,
    pub to_mint: String,
    pub from_amount: u64,
    pub to_amount: u64,
    pub jito_tip_amount: u64,
    pub wsol_change: Option<f64>,
    pub lamport_change: i64,
    pub decimals: u8,
}

impl ClassifiedTransaction {
    pub fn new() -> Self {
        ClassifiedTransaction {
            signature: String::new(),
            signer: String::new(),
            block_height: 0,
            block_time: None,
            instruction_type: String::new(),
            sandwich_acc: String::new(),
            swapper: String::new(),
            from_mint: String::new(),
            to_mint: String::new(),
            from_amount: 0,
            to_amount: 0,
            jito_tip_amount: 0,
            wsol_change: None,
            lamport_change: 0,
            decimals: 9, // Default to 9
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SwapInfo {
    pub swapper: String,
    pub from_mint: String,
    pub to_mint: String,
    pub from_amount: u64,
    pub to_amount: u64,
    pub wsol_change: Option<f64>,
    pub decimals: u8,
}

impl SwapInfo {
    pub fn new() -> Self {
        SwapInfo {
            swapper: String::new(),
            from_mint: String::new(),
            to_mint: String::new(),
            from_amount: 0,
            to_amount: 0,
            wsol_change: None,
            decimals: 9, // Default to 9
        }
    }
}

pub struct Pattern {
    pub token: String,
    pub attacker: String,
    pub victim: Option<String>,
    pub transactions: (ClassifiedTransaction, ClassifiedTransaction, ClassifiedTransaction),
}

impl Pattern {
    // Creates a new pattern from its component transactions
    pub fn new(
        create_tx: ClassifiedTransaction,
        swap_in_tx: ClassifiedTransaction,
        swap_out_tx: ClassifiedTransaction,
    ) -> Option<Self> {
        // Validate that all transactions have the same sandwich_acc
        if create_tx.sandwich_acc != swap_in_tx.sandwich_acc || swap_in_tx.sandwich_acc != swap_out_tx.sandwich_acc {
            return None;
        }

        // Validate the proper transaction sequence
        if create_tx.block_time > swap_in_tx.block_time || swap_in_tx.block_time > swap_out_tx.block_time {
            return None;
        }

        // Get the proper token from the swap transactions
        let token: String = if !swap_in_tx.from_mint.is_empty() {
            swap_in_tx.from_mint.clone()
        } else if !swap_out_tx.from_mint.is_empty() {
            swap_out_tx.from_mint.clone()
        } else {
            return None;
        };

        Some(Self {
            token,
            attacker: create_tx.signer.clone(),
            victim: Some(swap_in_tx.swapper.clone()),
            transactions: (create_tx, swap_in_tx, swap_out_tx),
        })
    }

    // Returns true if this is a profitable sandwich attack
    pub fn is_profitable(&self) -> bool {
        let (_, swap_in, swap_out) = &self.transactions;

        // Check if we have both swap amounts
        if swap_in.from_amount == 0 || swap_out.from_amount == 0 {
            return false;
        }

        swap_out.from_amount > swap_in.from_amount
    }

    // Returns true if this is a complete and valid sandwich attack pattern
    pub fn is_valid(&self) -> bool {
        let (create_tx, swap_in_tx, swap_out_tx) = &self.transactions;

        // Validate that all transactions use the same sandwich account
        if create_tx.sandwich_acc != swap_in_tx.sandwich_acc || swap_in_tx.sandwich_acc != swap_out_tx.sandwich_acc {
            return false;
        }

        // Validate transaction sequence is in the same block
        if create_tx.block_height != swap_in_tx.block_height || swap_in_tx.block_height != swap_out_tx.block_height {
            return false;
        }

        // Validate it's the same token
        if swap_in_tx.from_mint != swap_in_tx.to_mint || swap_out_tx.from_mint != swap_out_tx.to_mint {
            return false;
        }

        true
    }

    // Returns the token profit amount
    pub fn get_token_profit(&self) -> i128 {
        let (_, swap_in, swap_out) = &self.transactions;

        if !self.is_valid() {
            return 0;
        }

        swap_out.from_amount as i128 - swap_in.from_amount as i128
    }

    // Returns the SOL profit including both SOL and wSOL changes
    pub fn get_sol_profit(&self) -> f64 {
        let (_, swap_in_tx, swap_out_tx) = &self.transactions;

        let wsol_in: f64 = swap_in_tx.wsol_change.unwrap_or(0.0).abs(); // Positive (amount received)
        let wsol_out: f64 = swap_out_tx.wsol_change.unwrap_or(0.0).abs(); // Positive (amount sent)
        let jito_tip: f64 = swap_out_tx.jito_tip_amount as f64 / 1e9;
        let base_fees: f64 = 0.00001 * 2.0; // 2 base fees for in/out txs

        // Profit = Amount received - Amount sent - Jito tip - Base fees
        wsol_out - wsol_in - jito_tip - base_fees
    }

    // Returns a formatted string summarizing the pattern
    pub fn to_summary(&self) -> String {
        let token_profit: i128 = self.get_token_profit();
        let wsol_profit: f64 = self.get_sol_profit();
        let time_str: String = self
            .transactions
            .0
            .block_time
            .map(|t| {
                DateTime::<Utc>::from_timestamp(t as i64, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                    .unwrap_or_else(|| "Invalid timestamp".to_string())
            })
            .unwrap_or_else(|| "Unknown".to_string());
        let decimals: i32 = self.transactions.1.decimals.into();

        // Function to format token amounts using correct decimals
        let format_token_amount = |amount: i128| -> String {
            let decimal_divisor = 10_f64.powi(decimals as i32);
            format!("{:.6}", amount as f64 / decimal_divisor)
        };

        format!(
            "Sandwich Attack Pattern:\n\
             Token: {}\n\
             Token Profit: {} tokens\n\
             SOL Profit: {:.9} SOL\n\
             Attacker: {}\n\
             Victim: {}\n\
             Block Height: {}\n\
             Time: {}\n\
             Transactions:\n\
             - Create: {}\n\
             - Swap In: {} (amount: {})\n\
             - Swap Out: {} (amount: {})\n\
             Jito Tips Paid: {}\n",
            self.transactions.1.from_mint,
            format_token_amount(token_profit),
            wsol_profit,
            self.attacker,
            self.victim.as_ref().unwrap_or(&String::from("Unknown")),
            self.transactions.0.block_height,
            time_str,
            self.transactions.0.signature,
            self.transactions.1.signature,
            self.transactions.1.from_amount,
            self.transactions.2.signature,
            self.transactions.2.from_amount,
            self.transactions.2.jito_tip_amount,
        )
    }
}

// Tracks potential sandwich attacks in progress
#[derive(Default)]
pub struct PatternTracker {
    // Map of sandwich_acc -> create transaction
    open_positions: HashMap<String, ClassifiedTransaction>,
    // Map of sandwich_acc -> (create_tx, swap_in_tx)
    in_progress: HashMap<String, (ClassifiedTransaction, ClassifiedTransaction)>,
    // Completed patterns
    completed: Vec<Pattern>,
}

impl PatternTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn process_transaction(&mut self, tx: ClassifiedTransaction) {
        match tx.instruction_type.as_str() {
            "CreateSandwichV2" => {
                // Store create transaction indexed by sandwich account
                self.open_positions.insert(tx.sandwich_acc.clone(), tx);
            }
            "AutoSwapIn" => {
                // If we find a matching create transaction, move both to in_progress
                if let Some(create_tx) = self.open_positions.remove(&tx.sandwich_acc) {
                    self.in_progress.insert(tx.sandwich_acc.clone(), (create_tx, tx));
                }
            }
            "AutoSwapOut" => {
                // If we find matching in_progress transactions, try to create a pattern
                if let Some((create_tx, swap_in_tx)) = self.in_progress.remove(&tx.sandwich_acc) {
                    if let Some(pattern) = Pattern::new(create_tx, swap_in_tx, tx) {
                        self.completed.push(pattern);
                    }
                }
            }
            _ => {}
        }
    }

    pub fn get_completed_patterns(&self) -> &[Pattern] {
        &self.completed
    }

    pub fn clear_completed(&mut self) {
        self.completed.clear();
    }
}
