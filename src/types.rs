use std::collections::HashMap;

pub const TARGET_PROGRAM: &str = "vpeNALD89BZ4KxNUFjdLmFXBCwtyqBDQ85ouNoax38b";

// Discriminators are hex strings that identify each instruction
// The key is the hex discriminator, the value is the instruction name
pub fn get_instruction_map() -> HashMap<&'static str, &'static str> {
    let mut m: HashMap<&str, &str> = HashMap::new();

    m.insert("b3ecc1a00df8fe9a", "CreateSandwichV2");
    m.insert("5bb527f9eccb5e90", "AutoSwapIn");
    m.insert("b024faebda2bde25", "AutoSwapOut");
    m.insert("14d812f9d70bd653", "Cashout");
    m.insert("ea200c477e05dba0", "Exit");
    m.insert("7edb0b2a6825518b", "ExitPrice");
    m.insert("55e5a4f78f5c0591", "ExitInactivity");
    m.insert("b404ba74bbf3e278", "MigrateTokenData");

    m
}

#[allow(dead_code)]
pub struct ClassifiedTransaction {
    signature: String,
    signer: String,
    slot: u64,
    block_time: Option<u64>,
    instruction_type: String,
    sandwich_acc: String,
    from_mint: String,
    to_mint: String,
    from_amount: u64,
    to_amount: u64,
}

#[allow(dead_code)]
pub struct Pattern {
    token: String,
    attacker: String,
    victim: String,
    transactions: (ClassifiedTransaction, ClassifiedTransaction, ClassifiedTransaction),
}
