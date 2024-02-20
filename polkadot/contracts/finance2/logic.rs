
pub fn get_now(block_timestamp: u64, updated_at: u64) -> u64 {
    if block_timestamp < updated_at {
        updated_at
    } else {
        block_timestamp
    }
}