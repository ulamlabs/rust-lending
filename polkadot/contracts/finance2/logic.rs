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
        let x = result.low_u128();
        Some(add(x, c))
    }
}
pub fn ceil_up(a: U256, b: u128) -> Option<u128> {
    if b == 0 {
        None
    } else {
        let (result, rem) = a.div_mod(U256::from(b));
        if let Ok(x) = result.try_into() {
            let c = !rem.is_zero() as u128;
            Some(add(x, c))
        } else {
            None
        }
    }
}
pub fn scale(a: U256) -> u128 {
    use core::ops::Shr;
    a.shr(128).low_u128()
}
pub fn scale_up(a: U256) -> u128 {
    let c = !a.is_zero() as u128;
    add(scale(a), c)
}

/// We are not sure if now can be less than updated_at
/// It is possible, someone could accrue interest few times for the same period
/// Also integer overflow could occur and time delta calculation could wrap around
/// updated_at is updated here, to prevent using that function multiple time in the same message
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
    pub borrowed: u128,
    pub borrows: u128,
    pub liquidity: u128,
}
impl Quoter {
    pub fn quote(&self, collateral: u128) -> u128 {
        let w = mulw(collateral, self.price);
        div(w, self.price_scaler).unwrap_or(u128::MAX)
    }
    pub fn quote_debt(&self, borrowable: u128) -> u128 {
        let debt = sub(self.liquidity, borrowable);
    
        let user_debt = {
            let w = mulw(self.borrowed, debt);
            ceil_up(w, self.borrows).unwrap_or(debt)
        };
        {
            let w = mulw(user_debt, self.price);
            ceil_up(w, self.price_scaler).unwrap_or(u128::MAX)
        }
    }
    pub fn dequote(&self, discount: u128, qouted: u128) -> u128 {
        let price = {
            let w = mulw(self.price, discount);
            scale_up(w)
        };
        {
            let w = mulw(qouted, self.price_scaler);
            div(w, price).unwrap_or(u128::MAX)
        }
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
            scale_up(w)
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