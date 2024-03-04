use primitive_types::{U128, U256};

pub struct Wide(U256);

pub fn require<E>(cond: bool, e: E) -> Result<(), E> {
    if cond {
        Ok(())
    } else {
        Err(e)
    }
}

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
pub struct Accruer {
    pub now: u64,
    pub updated_at: u64,
    pub total_liquidity: u128,
    pub total_borrowable: u128,
    pub standard_rate: u128,
    pub emergency_rate: u128,
    pub standard_min_rate: u128,
    pub emergency_max_rate: u128,
}
impl Accruer {
    pub fn accrue(self) -> (u128, u64) {
        if self.now > self.updated_at {
            let delta = sub(self.now as u128, self.updated_at as u128);
            let standard_matured = self.standard_rate.saturating_mul(delta);
            let emergency_matured = self.emergency_rate.saturating_mul(delta);

            let debt = sub(self.total_liquidity, self.total_borrowable);

            let standard_scaled = mulw(standard_matured, debt).div_rate(self.total_liquidity).unwrap_or(0);
            let emergency_scaled = mulw(emergency_matured, self.total_borrowable).div_rate(self.total_liquidity).unwrap_or(0);

            let standard_final = standard_scaled.saturating_add(self.standard_min_rate);
            let emergency_final = self.emergency_max_rate.saturating_sub(emergency_scaled);

            let interest_rate = standard_final.max(emergency_final);
            let interest = mulw(debt, interest_rate).scale_up();

            let new_total_liquidity = self.total_liquidity.saturating_add(interest);
            (new_total_liquidity, self.now)    
        } else {
            (self.total_liquidity, self.updated_at)
        }
    }
}