use primitive_types::{U128, U256};

pub fn mulw(a: u128, b: u128) -> U256 {
    U128::from(a).full_mul(U128::from(b))
}
pub fn div_rate(a: U256, b: u128) -> Option<u128> {
    let r = a.checked_div(U256::from(b));
    r.map(|x| x.low_u128())
}
pub fn div(a: U256, b: u128) -> Option<u128> {
    let r = a.checked_div(U256::from(b));
    r.and_then(|x| x.try_into().ok())
}
pub fn add(a: u128, b: u128) -> u128 {
    a.wrapping_add(b)
}
pub fn sub(a: u128, b: u128) -> u128 {
    a.wrapping_sub(b)
}
pub fn ceil_rate(a: U256, b: u128) -> Option<u128> {
    if b == 0 {
        None
    } else {
        let (result, rem) = a.div_mod(U256::from(b));
        let c = !rem.is_zero() as u128;
        Some(add(result.low_u128(), c))
    }
}
pub fn scale(a: U256) -> u128 {
    use core::ops::Shr;
    a.shr(128).low_u128()
}

pub fn get_now(block_timestamp: u64, updated_at: u64) -> u64 {
    if block_timestamp < updated_at {
        updated_at
    } else {
        block_timestamp
    }
}

pub struct Quoter {
    pub price: u128,
    pub price_scaler: u128,
    pub collateral: u128,
    pub borrowed: u128,
    pub borrows: u128,
    pub liquidity: u128,
    pub borrowable: u128,
}
impl Quoter {
    pub fn quote(self) -> (u128, u128) {
        let quoted_collateral = {
            let w = mulw(self.collateral, self.price);
            div(w, self.price_scaler).unwrap_or(u128::MAX)
        };
        let debt = sub(self.liquidity, self.borrowable);

        let user_debt = {
            let w = mulw(self.borrowed, debt);
            div_rate(w, self.borrows).unwrap_or(0)
        };
        let quoted_debt = {
            let w = mulw(user_debt, self.price);
            div(w, self.price_scaler).unwrap_or(u128::MAX)
        };
        (quoted_collateral, quoted_debt)
    }
}