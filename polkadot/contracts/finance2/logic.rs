use primitive_types::{U128, U256};

pub struct Wide(U256);

pub fn mulw(a: u128, b: u128) -> Wide {
    let value = U128::from(a).full_mul(U128::from(b));
    Wide(value)
}
pub fn add(a: u128, b: u128) -> u128 {
    a.wrapping_add(b)
}
pub fn sub(a: u128, b: u128) -> u128 {
    a.wrapping_sub(b)
}
impl Wide {
    pub fn div_rate(&self, b: u128) -> Option<u128> {
        let r = self.0.checked_div(U256::from(b));
        r.map(|x| x.low_u128())
    }
    pub fn div(&self, b: u128) -> Option<u128> {
        let r = self.0.checked_div(U256::from(b));
        r.and_then(|x| x.try_into().ok())
    }
    
    pub fn ceil_rate(&self, b: u128) -> Option<u128> {
        if b == 0 {
            None
        } else {
            let (result, rem) = self.0.div_mod(U256::from(b));
            let c = !rem.is_zero() as u128;
            let x = result.low_u128();
            Some(add(x, c))
        }
    }
    pub fn ceil_up(&self, b: u128) -> Option<u128> {
        if b == 0 {
            None
        } else {
            let (result, rem) = self.0.div_mod(U256::from(b));
            if let Ok(x) = result.try_into() {
                let c = !rem.is_zero() as u128;
                Some(add(x, c))
            } else {
                None
            }
        }
    }
    pub fn scale(&self) -> u128 {
        use core::ops::Shr;
        self.0.shr(128).low_u128()
    }
    pub fn scale_up(&self) -> u128 {
        let c = !self.0.is_zero() as u128;
        add(self.scale(), c)
    }
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
        mulw(collateral, self.price).div(self.price_scaler).unwrap_or(u128::MAX)
    }
    pub fn quote_debt(&self, borrowable: u128) -> u128 {
        let debt = sub(self.liquidity, borrowable);
        let user_debt = mulw(self.borrowed, debt).ceil_up(self.borrows).unwrap_or(debt);

        mulw(user_debt, self.price).ceil_up(self.price_scaler).unwrap_or(u128::MAX)
    }
    pub fn dequote(&self, discount: u128, qouted: u128) -> u128 {
        let price = mulw(self.price, discount).scale_up();
        mulw(qouted, self.price_scaler).div(price).unwrap_or(u128::MAX)
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
            mulw(standard_matured, debt)
            .div_rate(self.liquidity)
            .unwrap_or(0)
        };
        let emergency_scaled = mulw(emergency_matured, self.borrowable)
            .div_rate(self.liquidity)
            .unwrap_or(0);

        let standard_final = standard_scaled.saturating_add(self.standard_min_rate);
        let emergency_final = self.emergency_max_rate.saturating_sub(emergency_scaled);

        let interest_rate = standard_final.max(emergency_final);
        let interest = mulw(debt, interest_rate).scale_up();

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
        let collateral_value = mulw(self.quoted_collateral, self.haircut).scale();
        
        let extra_debt = mulw(self.quoted_debt, self.margin).scale();
        let debt_value = self.quoted_debt.saturating_add(extra_debt);

        (collateral_value, debt_value)
    }
}