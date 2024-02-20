#![cfg_attr(not(feature = "std"), no_std, no_main)]

use ink::primitives::AccountId;

mod errors;
mod psp22;
mod logic;

#[cfg(test)]
mod tests;

#[ink::trait_definition]
pub trait LAsset {
    #[ink(message)]
    fn update(&mut self, user: AccountId) -> (AccountId, u128, u128);
}

#[ink::contract]
mod finance2 {
    use core::time;

    #[cfg(not(test))]
    use ink::contract_ref;
    use ink::prelude::vec::Vec;
    use ink::prelude::vec;
    use primitive_types::{U128, U256};
    use crate::logic;

    //Solving problem with small borrows/deposits
    const GAS_COLLATERAL: u128 = 1_000_000; // TODO find something less random

    fn mulw(a: u128, b: u128) -> U256 {
        U128::from(a).full_mul(U128::from(b))
    }
    fn div_rate(a: U256, b: u128) -> Option<u128> {
        let r = a.checked_div(U256::from(b));
        r.map(|x| x.low_u128())
    }
    fn div(a: U256, b: u128) -> Option<u128> {
        let r = a.checked_div(U256::from(b));
        r.and_then(|x| x.try_into().ok())
    }
    fn add(a: u128, b: u128) -> u128 {
        a.wrapping_add(b)
    }
    fn sub(a: u128, b: u128) -> u128 {
        a.wrapping_sub(b)
    }
    fn ceil_rate(a: U256, b: u128) -> Option<u128> {
        if b == 0 {
            None
        } else {
            let (result, rem) = a.div_mod(U256::from(b));
            let c = !rem.is_zero() as u128;
            Some(add(result.low_u128(), c))
        }
    }
    fn scale(a: U256) -> u128 {
        use core::ops::Shr;
        a.shr(128).low_u128()
    }
    // fn scale_up(a: U256) -> u128 {
    //     let c = !a.is_zero() as u128;
    //     add(scale(a), c)
    // }

    use ink::storage::Mapping;
    use crate::{errors::LAssetError, psp22::{PSP22Error, Transfer, Approval, PSP22}, LAsset};

    #[ink(storage)]
    pub struct LAssetContract {
        admin: AccountId,
        underlying_token: AccountId,
        updated_at: Timestamp,

        next: AccountId,

        collaterals: u128,
        collateral: Mapping<AccountId, u128>,
        
        //Maximum amount of liquidity that can be borrowed
        liquidity: u128,
        //Sum of all liquidity shares
        shares: u128,
        //Number of shares owned by each user
        share: Mapping<AccountId, u128>,
        allowance: Mapping<(AccountId, AccountId), u128>,
        
        //Amount of liquidity that can be borrowed
        //It is better to store it in that way, because
        //It is impossible to forget about check, that someone is borrowing to much
        //It has more optimal, becuase it does not have to be touched during updates
        borrowable: u128,
        //Sum of all borrow shares
        borrows: u128,
        //Number of shares owned by each user
        borrowed: Mapping<AccountId, u128>,

        standard_rate: u128,
        standard_min_rate: u128,

        emergency_rate: u128,
        emergency_max_rate: u128,

        initial_margin: u128,
        maintenance_margin: u128,

        initial_haircut: u128,
        maintenance_haircut: u128,

        price: u128,
        price_scaler: u128,
    }

    #[cfg(test)]
    static mut L_BTC: Option<LAssetContract> = None;
    #[cfg(test)]
    static mut L_USDC: Option<LAssetContract> = None;
    #[cfg(test)]
    static mut L_ETH: Option<LAssetContract> = None;


    impl LAssetContract {
        #[allow(clippy::too_many_arguments)]
        #[ink(constructor)]
        pub fn new(
            admin: AccountId,
            underlying_token: AccountId,
            next: AccountId,
            standard_rate: u128,
            standard_min_rate: u128,
            emergency_rate: u128,
            emergency_max_rate: u128,
            initial_margin: u128,
            maintenance_margin: u128,
            initial_haircut: u128,
            maintenance_haircut: u128,
            price: u128,
        ) -> Self {
            Self { 
                admin,
                underlying_token,
                updated_at: Self::env().block_timestamp(),
                next,
                collaterals: 0,
                collateral: Mapping::new(),
                liquidity: 0,
                shares: 0,
                share: Mapping::new(),
                allowance: Mapping::new(),
                borrowable: 0,
                borrows: 0,
                borrowed: Mapping::new(),
                standard_rate,
                standard_min_rate,
                emergency_rate,
                emergency_max_rate,
                initial_margin,
                maintenance_margin,
                initial_haircut,
                maintenance_haircut,
                price,
                price_scaler: 1,
             }
        }

        #[cfg(not(test))]
        fn update_next(&self, next: &AccountId, user: &AccountId) -> (AccountId, u128, u128) {
            let mut next: contract_ref!(LAsset) = (*next).into();
            next.update(*user)
        }

        #[cfg(test)]
        fn update_next(&self, next: &AccountId, user: &AccountId) -> (AccountId, u128, u128) {
            unsafe {
                if *next == AccountId::from([0x1; 32]) {
                    return L_BTC.as_mut().unwrap().update(*user);
                }
                if *next == AccountId::from([0x2; 32]) {
                    return L_USDC.as_mut().unwrap().update(*user);
                }
                if *next == AccountId::from([0x3; 32]) {
                    return L_ETH.as_mut().unwrap().update(*user);
                }
                unreachable!();
            }
        }

        #[cfg(not(test))]
        fn transfer_from_underlying(&self, token: AccountId, from: AccountId, to: AccountId, value: u128) -> Result<(), PSP22Error> {
            let mut token: contract_ref!(PSP22) = token.into();
            token.transfer_from(from, to, value, vec![])
        }
        #[cfg(test)]
        fn transfer_from_underlying(&self, token: AccountId, from: AccountId, to: AccountId, value: u128) -> Result<(), PSP22Error> {
            Ok(())
        }

        #[cfg(not(test))]
        fn transfer_underlying(&self, to: AccountId, value: u128) -> Result<(), PSP22Error> {
            let mut token: contract_ref!(PSP22) = self.underlying_token.into();
            token.transfer(to, value, vec![])
        }
        #[cfg(test)]
        fn transfer_underlying(&self, to: AccountId, value: u128) -> Result<(), PSP22Error> {
            Ok(())
        }

        fn update_all(&self, 
            mut next: AccountId, 
            current: &AccountId, 
            user: &AccountId,
            collateral_value: u128,
            debt_value: u128,
        ) -> Result<(), LAssetError> {
            let mut total_collateral_value = collateral_value;
            let mut total_debt_value = debt_value;
            while next != *current {
                //It is possible to reentract inside update_next, but it is not a problem
                //Because result of update_next is used only to accept the update
                //So if someone tries to trick this call, inside call must be tricked as well
                //As inside call is protected in the same way, at the end transaction will fail
                let (next2, collateral_value, debt_value) = self.update_next(&next, user);
                next = next2;
                total_collateral_value = total_collateral_value.saturating_add(collateral_value);
                total_debt_value = total_debt_value.saturating_add(debt_value);
            }
            if total_collateral_value < total_debt_value {
                Err(LAssetError::CollateralValueTooLow)
            } else {
                Ok(())
            }
        }

        fn increase_liquidity(
            &self,
            now: Timestamp, 
            updated_at: Timestamp,
            total_liquidity: u128,
            total_borrowable: u128,
            total_debt: u128,
        ) -> u128 {
            //impossible to overflow, because now >= updated_at
            let delta = sub(now as u128, updated_at as u128);
    
            //TODO: refactor to struct Rates and pass it by argument
            let standard_matured = self.standard_rate.saturating_mul(delta);
            let emergency_matured = self.emergency_rate.saturating_mul(delta);
    
            let standard_scaled = {
                let w = mulw(standard_matured, total_debt);
                div_rate(w, total_liquidity).unwrap_or(0)
            };
            let emergency_scaled = {
                let w = mulw(emergency_matured, total_borrowable);
                div_rate(w, total_liquidity).unwrap_or(0)
            };
    
            let standard_final = standard_scaled.saturating_add(self.standard_min_rate);
            let emergency_final = self.emergency_max_rate.saturating_sub(emergency_scaled);
    
            let interest_rate = standard_final.max(emergency_final);
            let interest = {
                let w = mulw(total_debt, interest_rate);
                scale(w)
            };
    
            total_liquidity.saturating_add(interest)
        }

        fn quote(&self, 
            user: &AccountId,
            now: Timestamp,
            updated_at: Timestamp,
            collateral: u128,
            borrowed: u128,
            borrows: u128,
            debt: u128,
        ) -> (u128, u128) {
            let price = self.price;
            let price_scaler = self.price_scaler;

            let quoted_collateral = {
                let w = mulw(collateral, price);
                div(w, price_scaler).unwrap_or(u128::MAX)
            };

            let total_borrow_shares = self.borrows;
            let user_debt = {
                let w = mulw(borrowed, debt);
                div_rate(w, total_borrow_shares).unwrap_or(0)
            };
            let quoted_debt = {
                let w = mulw(user_debt, price);
                div(w, price_scaler).unwrap_or(u128::MAX)
            };
            (quoted_collateral, quoted_debt)
        } 

        fn initial_values(
            &self,
            collateral_price: u128,
            debt_price: u128, 
        ) -> (u128, u128) {
            let margin = self.initial_margin;
            let haircut = self.initial_haircut;
            let collateral_value = {
                let w = mulw(collateral_price, haircut);
                scale(w)
            };
            let debt_delta = {
                let w = mulw(debt_price, margin);
                scale(w)
            };
            let debt_value = debt_price.saturating_add(debt_delta);
            (collateral_value, debt_value)
        }

        //We are not sure if now can be less than updated_at
        //It is possible, someone could accrue interest few times for the same period
        //Also integer overflow could occur and time delta calculation could wrap around
        //updated_at is updated here, to prevent using that function multiple time in the same message
        fn get_now(&self, updated_at: Timestamp) -> Timestamp {
            let now = self.env().block_timestamp();
            if now < updated_at {
                updated_at
            } else {
                now
            }
        }
        
        //There function does not require anything
        //Depositing collateral is absolutely independent
        //The only risk is that use will deposit small amount of tokens
        //And it's going to be hard to liquidate such user
        //We have to introduce somekind of gas collateral
        #[ink(message)]
        pub fn deposit(&mut self, amount: u128) -> Result<(), LAssetError> {
            //You can deposit for yourself only
            let caller = self.env().caller();
            let this = self.env().account_id();

            //To prevent reentrancy attack, we have to transfer tokens first
            //For example, if we have `let total_collateral = self.total_collateral`
            //And leter use it to update `total_collateral`, it would be possible
            //To reenter deposit function and update `total_collateral` using old, invalid value
            if let Err(e) = self.transfer_from_underlying(self.underlying_token, caller, this, amount) {
                Err(LAssetError::DepositTransferFailed(e))
            } else {
                Ok(())
            }?;

            //It is important to check if user collateral cannot be initialized in any other way
            //It would allow user to deposit without gas collateral
            let collateral = if let Some(c) = self.collateral.get(caller) {
                Ok(c)
            } else {
                let value = self.env().transferred_value();
                if value != GAS_COLLATERAL {
                    Err(LAssetError::FirstDepositRequiresGasCollateral)
                } else {
                    Ok(0)
                }
            }?;

            let new_total_collateral = {
                let r = self.collaterals.checked_add(amount);
                r.ok_or(LAssetError::DepositOverflow)
            }?;
            //Impossible to overflow, proofs/collateral.py for proof
            let new_collateral = add(collateral, amount);

            //it is crucial to update those two variables together
            self.collaterals = new_total_collateral;
            self.collateral.insert(caller, &new_collateral);

            Ok(())
        }

        //This function is very dangerous, because collateral is the only thing
        //That keep borrower from running away with borrowed liquidity
        //It is crucial to check if collateral value is greater than value of borrowed liquidity
        #[ink(message)]
        pub fn withdraw(&mut self, amount: u128) -> Result<(), LAssetError> {
            //You can withdraw for yourself only
            let caller = self.env().caller();

            //It is used to end recursion
            let this = self.env().account_id();

            let updated_at = self.updated_at;
            let now = self.get_now(updated_at);

            let collateral = {
                //It is important not to allow user to withdraw without deposit
                //It would allow user to deposit without gas collateral
                let r = self.collateral.get(caller);
                r.ok_or(LAssetError::WithdrawWithoutDeposit)
            }?;

            let new_collateral = {
                let r = collateral.checked_sub(amount);
                r.ok_or(LAssetError::WithdrawOverflow)
            }?;

            //We can ignore the fact, that user did not borrow anything yet, because
            //Borrow shares are not updated in this call
            let borrowed = self.borrowed.get(caller).unwrap_or(0);
            let total_borrowable = self.borrowable;

            //Impossible to overflow, proofs/collateral.py for proof
            let new_total_collateral = sub(self.collaterals, amount);

            let (quoted_collateral, quoted_debt, new_liquidity) = self.quote(&caller, now, updated_at, new_collateral, borrowed, total_borrowable);
            let (collateral_value, debt_value) = self.initial_values(quoted_collateral, quoted_debt);

            //Collateral must be updated before update
            //Inside update_all, we call next, so it is possible to reenter withdraw
            //Those values can be updated now, because update does not affect them
            self.collaterals = new_total_collateral;
            self.collateral.insert(caller, &new_collateral);

            //We can update those now, because, updating other pools does not affect them
            //If we do it later, it would be possible to reenter and currupt total liquidity state
            self.updated_at = now;
            self.liquidity = new_liquidity;

            self.update_all(self.next, &this, &caller, collateral_value, debt_value)?;

            //Transfer out after state is updated to prevent reentrancy attack
            //If someone tries to reenter, the most what can be achieved would be to change events emiting order
            if let Err(e) = self.transfer_underlying(caller, amount) {
                Err(LAssetError::WithdrawTransferFailed(e))
            } else {
                Ok(())
            }?;

            Ok(())
        }

        //In this function amount is number of underlying tokens, not shares
        //Number of minted shares depends on total liquidity and total shares
        #[ink(message)]
        pub fn mint(&mut self, amount: u128) -> Result<(), LAssetError> {
            //You can mint for yourself only
            let caller = self.env().caller();

            let this = self.env().account_id();

            //To prevent reentrancy attack, we have to transfer tokens first
            if let Err(e) = self.transfer_from_underlying(self.underlying_token, caller, this, amount) {
                Err(LAssetError::MintTransferFailed(e))
            } else {
                Ok(())
            }?;

            let updated_at = self.updated_at;
            let now = self.get_now(updated_at);
            let old_total_liquidity = self.liquidity;
            let total_borrowable = self.borrowable;
            let old_debt = sub(old_total_liquidity, total_borrowable);
            let total_liquidity = self.increase_liquidity(now, updated_at, old_total_liquidity, total_borrowable, old_debt);

            let total_shares = self.shares;
            //First mint does not require any extra actions
            let shares = self.share.get(caller).unwrap_or(0);

            let new_total_liquidity = {
                let r = total_liquidity.checked_add(amount);
                r.ok_or(LAssetError::MintLiquidityOverflow)
            }?;
            let minted = {
                let w = mulw(amount, total_shares);
                if let Some(m) = div_rate(w, total_liquidity) {
                    Ok(m)
                } else {
                    // First shares are scalled by 2^16. It limits total_shares to 2^112
                    if let Some(first_shares) = amount.checked_shl(16) {
                        Ok(first_shares)
                    } else {
                        Err(LAssetError::MintOverflow)
                    }
                }
            }?;
            
            //impossible to overflow IF total_liquidity is tracked correctly
            let new_shares = add(shares, minted);
            let new_total_shares = add(total_shares, minted);
            let new_total_borrowable = add(total_borrowable, amount);

            //it is crucial to update those four variables together
            self.liquidity = new_total_liquidity;
            self.shares = new_total_shares;
            self.share.insert(caller, &new_shares);
            self.borrowable = new_total_borrowable;

            self.updated_at = now;

            Ok(())
        }

        //in this function amount is number of shares, not underlying token
        #[ink(message)]
        pub fn burn(&mut self, amount: u128) -> Result<(), LAssetError> {
            //You can burn for yourself only
            let caller = self.env().caller();

            let updated_at = self.updated_at;
            let timestamp = self.env().block_timestamp();
            let now = logic::get_now(timestamp, updated_at);

            let old_liquidity = self.liquidity;
            let borrowable = self.borrowable;
            let old_debt = sub(old_liquidity, borrowable);

            let liquidity = self.increase_liquidity(now, updated_at, old_liquidity, borrowable, old_debt);

            let shares = self.shares;
            //Burn without mint is useless, but not forbidden
            let share = self.share.get(caller).unwrap_or(0);

            let new_share = if let Some(r) = share.checked_sub(amount) {
                Ok(r)
            } else {
                Err(LAssetError::BurnOverflow)
            }?;

            //Number of withdrawned liquidity is reduced by division precision
            //It is even possible to withdraw zero liquidity, even if some shares are burned
            //It has good sides, number of liquidity will never be grater than number of shares
            //And it incentives caller not to burn shares, but hold them longer
            let to_withdraw = {
                let w = mulw(amount, liquidity);
                div_rate(w, shares).unwrap_or(0)
            };

            //impossible to overflow IF liquidity_shares are tracked correctly
            let new_shares = sub(shares, amount);
            //impossible to overflow IF total_liquidity is tracked correctly
            let new_liquidity = sub(liquidity, to_withdraw);

            let new_borrowable = if let Some(r) = borrowable.checked_sub(to_withdraw) {
                Ok(r)
            } else {
                Err(LAssetError::BurnTooMuch)
            }?;

            //it is crucial to update those four variables together
            self.liquidity = new_liquidity;
            self.shares = new_shares;
            self.share.insert(caller, &new_share);
            self.borrowable = new_borrowable;

            self.updated_at = now;

            if let Err(e) = self.transfer_underlying(caller, to_withdraw) {
                Err(LAssetError::BurnTransferFailed(e))
            } else {
                Ok(())
            }?;

            Ok(())
        }

        //In this function amount is amount of liquidity, not shares
        #[ink(message)]
        pub fn borrow(&mut self, amount: u128) -> Result<(), LAssetError> {
            //You can borrow for yourself only
            let caller = self.env().caller();

            let updated_at = self.updated_at;
            let timestamp = self.env().block_timestamp();
            let now = logic::get_now(timestamp, updated_at);
            
            let next = self.next;
            let current = self.env().account_id();
            
            let borrowable = self.borrowable;
            let liquidity = self.liquidity;
            let old_debt = sub(liquidity, borrowable);            
            let new_liquidity = self.increase_liquidity(now, updated_at, liquidity, borrowable, old_debt);

            let borrows = self.borrows;
            let debt = sub(new_liquidity, borrowable);

            let new_borrowable = if let Some(r) = borrowable.checked_sub(amount) {
                Ok(r)
            } else {
                Err(LAssetError::BorrowableOverflow)
            }?;
            //Number of borrowed shares would be reduced by division precision
            //It is not wanted, because it would lead to situation, when
            //caller could borrow some liquidity without minting any shares
            //ceiling is solving that problem
            let minted = {
                let w = mulw(amount, borrows);
                if let Some(m) = ceil_rate(w, debt) {
                    Ok(m)
                } else {
                    // First minted are scaled by 2^16. It limits borrows to 2^112
                    if let Some(first_minted) = amount.checked_shl(16) {
                        Ok(first_minted)
                    } else {
                        Err(LAssetError::BurnOverflow)
                    }
                }
            }?;

            let borrowed = if let Some(borrowed) = self.borrowed.get(caller) {
                Ok(borrowed)
            } else {
                if self.env().transferred_value() != GAS_COLLATERAL {
                    Err(LAssetError::FirstBorrowRequiresGasCollateral)
                } else {
                    Ok(0)
                }
            }?;

            let collateral = self.collateral.get(caller).unwrap_or(0);
            
            let new_borrowed = add(borrowed, minted);
            let new_borrows = add(borrows, minted);
            let new_debt = sub(new_liquidity, new_borrowable);
            
            let (quoted_collateral, quoted_debt) = self.quote(&caller, now, updated_at, collateral, new_borrowed, new_borrows, new_debt);

            let (collateral_value, debt_value) = self.initial_values(quoted_collateral, quoted_debt);
            
            //it is crucial to update those three variables together
            self.borrowable = new_borrowable;
            self.borrows = new_borrows;
            self.borrowed.insert(caller, &new_borrowed);

            self.liquidity = new_liquidity;
            self.updated_at = now;

            self.update_all(next, &current, &caller, collateral_value, debt_value)?;

            if let Err(e) = self.transfer_underlying(caller, amount) {
                Err(LAssetError::BorrowTransferFailed(e))
            } else {
                Ok(())
            }?;

            Ok(())
        }

        #[ink(message)]
        pub fn repay(&mut self, amount: u128) -> Result<(), LAssetError> {
            //You can repay for yourself only
            let caller = self.env().caller();

            let updated_at = self.updated_at;
            let timestamp = self.env().block_timestamp();
            let now = logic::get_now(timestamp, updated_at);

            let borrowed = if let Some(r) = self.borrowed.get(caller) {
                Ok(r)
            } else {
                Err(LAssetError::RepayWithoutBorrow)
            }?;

            let new_borrowed = if let Some(r) = borrowed.checked_sub(amount) {
                Ok(r)
            } else {
                Err(LAssetError::RepayOverflow)
            }?;

            let liquidity = self.liquidity;
            let borrowable = self.borrowable;
            let debt = sub(liquidity, borrowable);
            let new_liquidity = self.increase_liquidity(now, updated_at, liquidity, borrowable, debt);
            
            let new_debt = sub(new_liquidity, borrowable);
            let borrows = self.borrows;
            let repayed = {
                let w = mulw(amount, debt);
                div_rate(w, borrows).unwrap_or(0)
            };

            let new_borrowable = add(borrowable, repayed);
            let new_borrows = sub(borrows, amount);

            //it is crucial to update those three variables together
            self.borrowable = new_borrowable;
            self.borrows = new_borrows;
            self.borrowed.insert(caller, &new_borrowed);

            self.liquidity = new_liquidity;
            self.updated_at = now;

            Ok(())
        }
    }

    impl LAsset for LAssetContract {
        #[ink(message)]
        fn update(&mut self, user: AccountId) -> (AccountId, u128, u128) {
            let updated_at = self.updated_at;
            let now = self.get_now(updated_at);
            let next = self.next;
            let user_collateral = self.collateral.get(user).unwrap_or(0);
            
            let (collateral, debt, new_liquidity) = self.quote(&user, now, updated_at, user_collateral);
            let (collateral_value, debt_value) = self.initial_values(collateral, debt);

            self.updated_at = now;
            self.liquidity = new_liquidity;

            (next, collateral_value, debt_value)
        }
    }

    impl PSP22 for LAssetContract {
        #[ink(message)]
        fn total_supply(&self) -> u128 {
            self.shares
        }

        #[ink(message)]
        fn balance_of(&self, owner: AccountId) -> u128 {
            self.share.get(owner).unwrap_or(0)
        }

        #[ink(message)]
        fn allowance(&self, owner: AccountId, spender: AccountId) -> u128 {
            self.allowance.get((owner, spender)).unwrap_or(0)
        }

        #[ink(message)]
        fn transfer(&mut self, to: AccountId, value: u128, _data: Vec<u8>) -> Result<(), PSP22Error> {
            let from = self.env().caller();
            let from_shares = self.share.get(from).unwrap_or(0);
            let to_shares = self.share.get(to).unwrap_or(0);

            let new_from_shares = {
                let r = from_shares.checked_sub(value);
                r.ok_or(PSP22Error::InsufficientBalance)
            }?;
            let new_to_shares = add(to_shares, value);
            let event = Transfer {
                from: Some(from), 
                to: Some(to), 
                value,
            };

            self.share.insert(from, &new_from_shares);
            self.share.insert(to, &new_to_shares);

            self.env().emit_event(event);
            Ok(())
        }

        #[ink(message)]
        fn transfer_from(&mut self, from: AccountId, to: AccountId, value: u128, _data: Vec<u8>) -> Result<(), PSP22Error> {
            let from_shares = self.share.get(from).unwrap_or(0);
            let to_shares = self.share.get(to).unwrap_or(0);
            let allowance = self.allowance.get((from, to)).unwrap_or(0);
            
            let new_allowance = {
                let r = allowance.checked_sub(value);
                r.ok_or(PSP22Error::InsufficientAllowance)
            }?;
            let new_from_shares = {
                let r = from_shares.checked_sub(value);
                r.ok_or(PSP22Error::InsufficientBalance)
            }?;
            let new_to_shares = add(to_shares, value);
            let approval_event = Approval {
                owner: from, 
                spender: to, 
                amount: new_allowance
            };
            let transfer_event = Transfer {
                from: Some(from), 
                to: Some(to), 
                value,
            };

            self.share.insert(from, &new_from_shares);
            self.share.insert(to, &new_to_shares);
            self.allowance.insert((from, to), &new_allowance);

            self.env().emit_event(approval_event);
            self.env().emit_event(transfer_event);
            Ok(())
        }

        #[ink(message)]
        fn approve(&mut self, spender: AccountId, value: u128) -> Result<(), PSP22Error> {
            let owner = self.env().caller();
            
            let event = Approval {
                owner, 
                spender, 
                amount: value
            };

            self.allowance.insert((owner, spender), &value);
            
            self.env().emit_event(event);
            Ok(())
        }

        #[ink(message)]
        fn increase_allowance(&mut self, spender: AccountId, delta_value: u128) -> Result<(), PSP22Error> {
            let owner = self.env().caller();
            let allowance = self.allowance.get((owner, spender)).unwrap_or(0);
            
            let new_allowance = allowance.saturating_add(delta_value);
            let event = Approval {
                owner, 
                spender, 
                amount: new_allowance
            };
            
            self.allowance.insert((owner, spender), &new_allowance);

            self.env().emit_event(event);
            Ok(())
        }

        #[ink(message)]
        fn decrease_allowance(&mut self, spender: AccountId, delta_value: u128) -> Result<(), PSP22Error> {
            let owner = self.env().caller();
            let allowance = self.allowance.get((owner, spender)).unwrap_or(0);
            
            let new_allowance = allowance.saturating_sub(delta_value);
            let event = Approval {
                owner, 
                spender, 
                amount: new_allowance
            };

            self.allowance.insert((owner, spender), &new_allowance);

            self.env().emit_event(event);
            Ok(())
        }
    }    
}
