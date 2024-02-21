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

pub struct Accruer {
    pub now: u64,
    pub updated_at: u64,
    pub liquidity: u128,
    pub borrowable: u128,
    pub standard_rate: u128,
    pub emergency_rate: u128,
    pub standard_min_rate: u128,
    pub emergency_max_rate: u128,
}
impl Accruer {
    pub fn accrue(self) -> u128 {
        let delta = sub(self.now as u128, self.updated_at as u128);
        let standard_matured = self.standard_rate.saturating_mul(delta);
        let emergency_matured = self.emergency_rate.saturating_mul(delta);

        let debt = sub(self.liquidity, self.borrowable);

        let standard_scaled = {
            let w = mulw(standard_matured, debt);
            div_rate(w, self.liquidity).unwrap_or(0)
        };
        let emergency_scaled = {
            let w = mulw(emergency_matured, self.borrowable);
            div_rate(w, self.liquidity).unwrap_or(0)
        };

        let standard_final = standard_scaled.saturating_add(self.standard_min_rate);
        let emergency_final = self.emergency_max_rate.saturating_sub(emergency_scaled);

        let interest_rate = standard_final.max(emergency_final);
        let interest = {
            let w = mulw(debt, interest_rate);
            scale(w)
        };

        self.liquidity.saturating_add(interest)
    }
}

pub struct Valuator {
    pub margin: u128,
    pub haircut: u128,
    pub quoted_collateral: u128,
    pub quoted_debt: u128,
}
impl Valuator {
    pub fn values(self) -> (u128, u128) {
        let collateral_value = {
            let w = mulw(self.quoted_collateral, self.haircut);
            scale(w)
        };
        let debt_delta = {
            let w = mulw(self.quoted_debt, self.margin);
            scale(w)
        };
        let debt_value = self.quoted_debt.saturating_add(debt_delta);
        (collateral_value, debt_value)
    }
}