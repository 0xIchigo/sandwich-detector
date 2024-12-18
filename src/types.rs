use serde::Serialize;
use std::collections::HashMap;

pub const MIN_JITO_TIP: u64 = 1000;
pub const TARGET_PROGRAM: &str = "vpeNALD89BZ4KxNUFjdLmFXBCwtyqBDQ85ouNoax38b";

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
    pub slot: u64,
    pub block_time: Option<u64>,
    pub instruction_type: String,
    pub sandwich_acc: String,
    pub swapper: String,
    pub from_mint: String,
    pub to_mint: String,
    pub from_amount: u64,
    pub to_amount: u64,
    pub jito_tip_amount: u64,
}

impl ClassifiedTransaction {
    pub fn new() -> Self {
        ClassifiedTransaction {
            signature: String::new(),
            signer: String::new(),
            slot: 0,
            block_time: None,
            instruction_type: String::new(),
            sandwich_acc: String::new(),
            swapper: String::new(),
            from_mint: String::new(),
            to_mint: String::new(),
            from_amount: 0,
            to_amount: 0,
            jito_tip_amount: 0,
        }
    }
}

#[allow(dead_code)]
pub struct Pattern {
    pub token: String,
    pub attacker: String,
    pub victim: Option<String>,
    pub transactions: (ClassifiedTransaction, ClassifiedTransaction, ClassifiedTransaction),
}

#[derive(Debug, Serialize)]
pub struct SwapInfo {
    pub swapper: String,
    pub from_mint: String,
    pub to_mint: String,
    pub from_amount: u64,
    pub to_amount: u64,
}

impl SwapInfo {
    pub fn new() -> Self {
        SwapInfo {
            swapper: String::new(),
            from_mint: String::new(),
            to_mint: String::new(),
            from_amount: 0,
            to_amount: 0,
        }
    }
}
