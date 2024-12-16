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
