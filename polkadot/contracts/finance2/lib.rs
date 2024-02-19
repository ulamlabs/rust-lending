#![cfg_attr(not(feature = "std"), no_std, no_main)]

use ink::primitives::AccountId;

mod errors;
mod psp22;

#[cfg(test)]
mod tests;

#[ink::trait_definition]
pub trait LAsset {
    #[ink(message)]
    fn update(&mut self, user: AccountId) -> (AccountId, u128, u128);
}

#[ink::contract]
mod finance2 {
    #[cfg(not(test))]
    use ink::contract_ref;
    use ink::prelude::vec::Vec;
    use primitive_types::{U128, U256};

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
    // fn ceil_rate(a: U256, b: u128) -> Option<u128> {
    //     if b == 0 {
    //         None
    //     } else {
    //         let (result, rem) = a.div_mod(U256::from(b));
    //         let c = !rem.is_zero() as u128;
    //         Some(add(result.low_u128(), c))
    //     }
    // }
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
        asset: AccountId,
        updated_at: Timestamp,

        next: AccountId,
        prev: AccountId,

        total_collateral: u128,
        collaterals: Mapping<AccountId, u128>,
        
        //Maximum amount of liquidity that can be borrowed
        total_liquidity: u128,
        //Sum of all liquidity shares
        total_liquidity_shares: u128,
        //Number of shares owned by each user
        liquidity_shares: Mapping<AccountId, u128>,
        allowance: Mapping<(AccountId, AccountId), u128>,
        
        //Amount of liquidity that can be borrowed
        //It is better to store it in that way, because
        //It is impossible to forget about check, that someone is borrowing to much
        //It has more optimal, becuase it does not have to be touched during updates
        total_borrowable: u128,
        //Sum of all borrow shares
        total_borrow_shares: u128,
        //Number of shares owned by each user
        borrow_shares: Mapping<AccountId, u128>,

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
            asset: AccountId,
            next: AccountId,
            prev: AccountId,
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
                asset,
                updated_at: Self::env().block_timestamp(),
                next,
                prev,
                total_collateral: 0,
                collaterals: Mapping::new(),
                total_liquidity: 0,
                total_liquidity_shares: 0,
                liquidity_shares: Mapping::new(),
                allowance: Mapping::new(),
                total_borrowable: 0,
                total_borrow_shares: 0,
                borrow_shares: Mapping::new(),
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

        fn update_me(
            &self,
            total_borrowable: u128,
        ) -> (Timestamp, u128) {
            let updated_at = self.updated_at;
            let now = self.get_now(updated_at);
            let total_liquidity = self.total_liquidity;
            let total_debt = sub(total_liquidity, total_borrowable);
            let new_liquidity = self.increase_liquidity(
                now, 
                updated_at,
                total_liquidity,
                total_borrowable,
                total_debt,
            );
            (now, new_liquidity)
        }

        fn update_values(&self, 
            mut next: AccountId, 
            current: &AccountId, 
            user: &AccountId,
            user_collateral: u128,
        ) -> Result<(Timestamp, u128), LAssetError> {
            let updated_at = self.updated_at;
            let now = self.get_now(updated_at);
            let (collateral, debt, new_liquidity) = self.calculate_values(user, now, updated_at, user_collateral);
            let (collateral_value, debt_value) = self.calculate_initial_values(collateral, debt);
            
            let mut total_collateral_value = collateral_value;
            let mut total_debt_value = debt_value;
            while next != *current {
                let (next2, collateral_value, debt_value) = self.update_next(&next, user);
                next = next2;
                total_collateral_value = total_collateral_value.saturating_add(collateral_value);
                total_debt_value = total_debt_value.saturating_add(debt_value);
            }
            if total_collateral_value < total_debt_value {
                Err(LAssetError::CollateralValueTooLow)
            } else {
                Ok((now, new_liquidity))
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
            let standard_rate = self.standard_rate;
            let standard_min_rate = self.standard_min_rate;
            let emergency_rate = self.emergency_rate;
            let emergency_max_rate = self.emergency_max_rate;
    
            //impossible to overflow, because now > updated_at
            let delta = sub(now as u128, updated_at as u128);
    
            let standard_matured = standard_rate.saturating_mul(delta);
            let emergency_matured = emergency_rate.saturating_mul(delta);
    
            let standard_scaled = {
                let w = mulw(standard_matured, total_debt);
                div_rate(w, total_liquidity).unwrap_or(0)
            };
            let emergency_scaled = {
                let w = mulw(emergency_matured, total_borrowable);
                div_rate(w, total_liquidity).unwrap_or(0)
            };
    
            let standard_final = standard_scaled.saturating_add(standard_min_rate);
            let emergency_final = emergency_max_rate.saturating_sub(emergency_scaled);
    
            let interest_rate = standard_final.max(emergency_final);
            let interest = {
                let w = mulw(total_debt, interest_rate);
                scale(w)
            };
    
            total_liquidity.saturating_add(interest)
        }

        fn calculate_values(&self, 
            user: &AccountId,
            now: Timestamp,
            updated_at: Timestamp,
            collateral: u128,
        ) -> (u128, u128, u128) {
            let price = self.price;
            let price_scaler = self.price_scaler;

            let collateral_value = {
                let w = mulw(collateral, price);
                div(w, price_scaler).unwrap_or(u128::MAX)
            };

            let total_liquidity = self.total_liquidity;
            let total_borrowable = self.total_borrowable;
            let total_debt = sub(total_liquidity, total_borrowable);
            let borrow_shares = self.borrow_shares.get(user).unwrap_or(0);
            let total_borrow_shares = self.total_borrow_shares;
            let debt_value = {
                let w = mulw(borrow_shares, total_debt);
                div_rate(w, total_borrow_shares).unwrap_or(0)
            };
            
            let new_liquidity = self.increase_liquidity(
                now, 
                updated_at,
                total_liquidity,
                total_borrowable,
                total_debt,
            );
            (collateral_value, debt_value, new_liquidity)
        } 

        fn calculate_initial_values(
            &self,
            collateral: u128,
            debt: u128, 
        ) -> (u128, u128) {
            let margin = self.initial_margin;
            let haircut = self.initial_haircut;
            let collateral_value = {
                let w = mulw(collateral, haircut);
                scale(w)
            };
            let debt_delta = {
                let w = mulw(debt, margin);
                scale(w)
            };
            let debt_value = debt.saturating_add(debt_delta);
            (collateral_value, debt_value)
        }

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
        #[ink(message)]
        pub fn deposit(&mut self, amount: u128) -> Result<(), LAssetError> {
            let env = self.env();
            //You can deposit for yourself only
            let caller = env.caller();

            let old_total_collateral = self.total_collateral;
            //First deposit does not require any extra actions
            let old_collateral = self.collaterals.get(caller).unwrap_or(0);

            let new_total_collateral = {
                //new_total_collateral is calculated first, because if it doesn't overflow,
                //it is impossible for new_collateral to overflow
                let r = old_total_collateral.checked_add(amount);

                //This check is potential blocker, but it can fail only if
                //token as total supply greater than 2^128
                //We cannot do anything about it, so we just return error
                r.ok_or(LAssetError::DepositOverflow)
            }?;
            //impossible to overflow IF total_collateral is tracked correctly
            let new_collateral = add(old_collateral, amount);

            //it is crucial to update those two variables together
            self.total_collateral = new_total_collateral;
            self.collaterals.insert(caller, &new_collateral);

            Ok(())
        }

        //This function is very dangerous, because collateral is the only thing
        //That keep borrower from running away with borrowed liquidity
        //It is crucial to check if collateral value is greater than value of borrowed liquidity
        #[ink(message)]
        pub fn withdraw(&mut self, amount: u128) -> Result<(), LAssetError> {
            //You can withdraw for yourself only
            let caller = self.env().caller();

            //It will be needed to update values of other assets
            let next = self.next;

            //It is used to end recursion
            let current = self.env().account_id();

            let total_collateral = self.total_collateral;
            //Withdraw without deposit is useless, but not forbidden
            let collateral = self.collaterals.get(caller).unwrap_or(0);

            let new_collateral = {
                //new_collateral is calculated first, because if it doesn't overflow,
                //it is impossible for new_total_collateral to overflow
                let r = collateral.checked_sub(amount);

                //This check is potential blocker, but it can fail only if
                //caller tries to withdraw more than she has
                r.ok_or(LAssetError::WithdrawOverflow)
            }?;

            let (now, new_liquidity) = self.update_values(next, &current, &caller, new_collateral)?;

            //impossible to overflow IF total_collateral is tracked correctly
            let new_total_collateral = sub(total_collateral, amount);


            //it is crucial to update those two variables together
            self.total_collateral = new_total_collateral;
            self.collaterals.insert(caller, &new_collateral);

            self.updated_at = now;
            self.total_liquidity = new_liquidity;
            Ok(())
        }

        //In this function amount is amount of liquidity, not shares
        //it is hard to predict how much shares will be minted
        #[ink(message)]
        pub fn mint(&mut self, amount: u128) -> Result<(), LAssetError> {
            let env = self.env();
            //You can mint for yourself only
            let caller = env.caller();

            let total_borrowable = self.total_borrowable;
            let (now, total_liquidity) = self.update_me(total_borrowable);

            let total_shares = self.total_liquidity_shares;
            //First mint does not require any extra actions
            let shares = self.liquidity_shares.get(caller).unwrap_or(0);

            let new_total_liquidity = {
                //new_total_liquidity is calculated first, because if it doesn't overflow,
                //it is impossible for new_shares and new_total_shares to overflow
                let r = total_liquidity.checked_add(amount);

                //This check is potential blocker, but it can fail only if
                //token as total supply greater than 2^128 OR
                //interest is enourmous, then only burn can help
                r.ok_or(LAssetError::MintOverflow)
            }?;
            //Number of minted shares is reduced by division precision
            //It is even possible to mint zero shares, even if some liquidity is added
            //It has good sides, number of shares will never be grater than number of liquidity
            //And it incentives caller to mint more liquidity at once
            //Early minters will get more shares, so it is incentive to hold shares longer
            let minted = {
                let w = mulw(shares, total_shares);
                div_rate(w, total_liquidity).unwrap_or(0)
            };
            //impossible to overflow IF total_liquidity is tracked correctly
            let new_shares = add(shares, minted);
            let new_total_shares = add(total_shares, minted);
            let new_total_borrowable = add(total_borrowable, amount);

            //it is crucial to update those four variables together
            self.total_liquidity = new_total_liquidity;
            self.total_liquidity_shares = new_total_shares;
            self.liquidity_shares.insert(caller, &new_shares);
            self.total_borrowable = new_total_borrowable;

            self.updated_at = now;

            Ok(())
        }

        //in this function amount is amount of shares, not liquidity
        //it is hard to predict how much liquidity will be burned
        //because someone else can mint or burn in the meantime
        #[ink(message)]
        pub fn burn(&mut self, amount: u128) -> Result<(), LAssetError> {
            let env = self.env();
            //You can burn for yourself only
            let caller = env.caller();

            let total_borrowable = self.total_borrowable;
            let (now, total_liquidity) = self.update_me(total_borrowable);

            let total_shares = self.total_liquidity_shares;
            //Burn without mint is useless, but not forbidden
            let shares = self.liquidity_shares.get(caller).unwrap_or(0);

            let new_shares = {
                //new_shares is calculated first, because if it doesn't overflow,
                //it is impossible for new_total_shares to overflow
                let r = shares.checked_sub(amount);

                //This check is potential blocker, but it can fail only if
                //caller tries to burn more than she has
                r.ok_or(LAssetError::BurnOverflow)
            }?;

            //Number of withdrawned liquidity is reduced by division precision
            //It is even possible to withdraw zero liquidity, even if some shares are burned
            //It has good sides, number of liquidity will never be grater than number of shares
            //And it incentives caller not to burn shares, but hold them longer
            let withdrawn = {
                let w = mulw(amount, total_liquidity);
                div_rate(w, total_shares).unwrap_or(0)
            };

            //impossible to overflow IF liquidity_shares are tracked correctly
            let new_total_shares = sub(total_shares, amount);
            //impossible to overflow IF total_liquidity is tracked correctly
            let new_total_liquidity = sub(total_liquidity, withdrawn);

            let new_total_borrowable = {
                let r = total_borrowable.checked_sub(withdrawn);

                //This check is potential blocker, but it can fail only if
                //Amount of borrowable goes below zero
                //User should accept the rist, that liquidity used to mint shares
                //Can be borrowed and impossible to withdraw
                //It is expected and if it happend, interest rate should be really high
                //And should soon lead to liquidation
                r.ok_or(LAssetError::BurnTooMuch)
            }?;

            //it is crucial to update those four variables together
            self.total_liquidity = new_total_liquidity;
            self.total_liquidity_shares = new_total_shares;
            self.liquidity_shares.insert(caller, &new_shares);
            self.total_borrowable = new_total_borrowable;

            self.updated_at = now;

            Ok(())
        }

        //In this function amount is amount of liquidity, not shares
        #[ink(message)]
        pub fn borrow(&mut self, amount: u128) -> Result<(), LAssetError> {
            let env = self.env();
            //You can borrow for yourself only
            let caller = env.caller();
            
            let next = self.next;
            let current = self.env().account_id();
            
            let user_collateral = self.collaterals.get(caller).unwrap_or(0);
            let (now, new_liquidity) = self.update_values(next, &current, &caller, user_collateral)?;
            
            let borrowable = self.total_borrowable;
            let total_shares = self.total_borrow_shares;
            //First borrow does not require any extra actions
            let shares = self.borrow_shares.get(caller).unwrap_or(0);

            let new_borrowable = {
                //new_borrowable is calculated first, because if it doesn't overflow,
                //it is impossible for new_shares and new_total_shares to overflow
                let r = borrowable.checked_sub(amount);

                //This check is potential blocker, but it can fail only if
                //caller tries to borrow more than it is possible
                r.ok_or(LAssetError::BorrowOverflow)
            }?;

            //impossible to overflow IF total_liquidity and borrowable are tracked correctly
            let total_debt = sub(new_liquidity, borrowable);

            //Number of borrowed shares would be reduced by division precision
            //It is not wanted, because it would lead to situation, when
            //caller could borrow some liquidity without minting any shares
            //ceiling is solving that problem
            let minted = {
                let w = mulw(amount, total_shares);
                div_rate(w, total_debt).unwrap_or(0)
            };
            let new_shares = add(shares, minted);
            let new_total_shares = add(total_shares, minted);
            
            //it is crucial to update those three variables together
            self.total_borrowable = new_borrowable;
            self.total_borrow_shares = new_total_shares;
            self.borrow_shares.insert(caller, &new_shares);

            self.total_liquidity = new_liquidity;
            self.updated_at = now;

            Ok(())
        }

        #[ink(message)]
        pub fn repay(&mut self, amount: u128) -> Result<(), LAssetError> {
            let env = self.env();
            //You can repay for yourself only
            let caller = env.caller();

            let borrowable = self.total_borrowable;
            let (now, new_liquidity) = self.update_me(borrowable);

            let total_shares = self.total_borrow_shares;
            //Repay without borrow is useless, but not forbidden
            let shares = self.borrow_shares.get(caller).unwrap_or(0);

            let new_shares = {
                //new_shares is calculated first, because if it doesn't overflow,
                //it is impossible for new_total_shares to overflow
                let r = shares.checked_sub(amount);

                //This check is potential blocker, but it can fail only if
                //caller tries to repay more than she has borrowed
                r.ok_or(LAssetError::RepayOverflow)
            }?;

            //impossible to overflow IF total_liquidity and borrowable are tracked correctly
            let total_debt = sub(new_liquidity, borrowable);
            
            //Number of repayed liquidity is reduced by division precision
            //It is not wanted, because it would lead to situation, when
            //caller could burn some borrow shares without repaying any debt
            //ceiling is solving that problem
            let repayed = {
                let w = mulw(amount, total_debt);
                div_rate(w, total_shares).unwrap_or(0)
            };

            let new_total_borrowable = add(borrowable, repayed);
            let new_total_shares = sub(total_shares, amount);

            //it is crucial to update those three variables together
            self.total_borrowable = new_total_borrowable;
            self.total_borrow_shares = new_total_shares;
            self.borrow_shares.insert(caller, &new_shares);

            self.total_liquidity = new_liquidity;
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
            let user_collateral = self.collaterals.get(user).unwrap_or(0);
            
            let (collateral, debt, new_liquidity) = self.calculate_values(&user, now, updated_at, user_collateral);
            let (collateral_value, debt_value) = self.calculate_initial_values(collateral, debt);

            self.updated_at = now;
            self.total_liquidity = new_liquidity;

            (next, collateral_value, debt_value)
        }
    }

    impl PSP22 for LAssetContract {
        #[ink(message)]
        fn total_supply(&self) -> u128 {
            self.total_liquidity_shares
        }

        #[ink(message)]
        fn balance_of(&self, owner: AccountId) -> u128 {
            self.liquidity_shares.get(owner).unwrap_or(0)
        }

        #[ink(message)]
        fn allowance(&self, owner: AccountId, spender: AccountId) -> u128 {
            self.allowance.get((owner, spender)).unwrap_or(0)
        }

        #[ink(message)]
        fn transfer(&mut self, to: AccountId, value: u128, _data: Vec<u8>) -> Result<(), PSP22Error> {
            let from = self.env().caller();
            let from_shares = self.liquidity_shares.get(from).unwrap_or(0);
            let to_shares = self.liquidity_shares.get(to).unwrap_or(0);

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

            self.liquidity_shares.insert(from, &new_from_shares);
            self.liquidity_shares.insert(to, &new_to_shares);

            self.env().emit_event(event);
            Ok(())
        }

        #[ink(message)]
        fn transfer_from(&mut self, from: AccountId, to: AccountId, value: u128, _data: Vec<u8>) -> Result<(), PSP22Error> {
            let from_shares = self.liquidity_shares.get(from).unwrap_or(0);
            let to_shares = self.liquidity_shares.get(to).unwrap_or(0);
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

            self.liquidity_shares.insert(from, &new_from_shares);
            self.liquidity_shares.insert(to, &new_to_shares);
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
