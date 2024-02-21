#![cfg_attr(not(feature = "std"), no_std, no_main)]

use errors::LAssetError;
use ink::primitives::AccountId;

mod errors;
mod psp22;
mod logic;

#[cfg(test)]
mod tests;

#[ink::trait_definition]
pub trait LAsset {
    #[ink(message)]
    fn update(&mut self, user: AccountId) -> (AccountId, u128, u128, u128, u128);

    #[ink(message)]
    fn repay(&mut self, user: AccountId, amount: u128, cash: u128, cash_owner: AccountId) -> Result<(AccountId, u128, u128, u128, u128, u128), LAssetError>;
}


#[ink::contract]
mod finance2 {
    #[cfg(not(test))]
    use ink::contract_ref;
    use ink::prelude::vec::Vec;
    use ink::prelude::vec;
    use crate::logic::{self, add, ceil_rate, div_rate, mulw, scale, sub};

    //Solving problem with small borrows/deposits
    const GAS_COLLATERAL: u128 = 1_000_000; // TODO find something less random
    const DEFAULT_DECIMALS: u8 = 6;

    // fn scale_up(a: U256) -> u128 {
    //     let c = !a.is_zero() as u128;
    //     add(scale(a), c)
    // }

    use ink::storage::Mapping;
    use crate::LAsset;
    use crate::errors::LAssetError;
    use crate::psp22::{PSP22Error, PSP22Metadata, Transfer, Approval, PSP22};

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

        cash: Mapping<AccountId, u128>,

        // PSP22Metadata
        name: Option<String>,
        symbol: Option<String>,
        decimals: u8,
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
            let (name, symbol, decimals) = fetch_psp22_metadata(underlying_token);

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
                cash: Mapping::new(),
                name,
                symbol,
                decimals,
             }
        }

        #[cfg(not(test))]
        fn update_next(&self, next: &AccountId, user: &AccountId) -> (AccountId, u128, u128, u128, u128) {
            let mut next: contract_ref!(LAsset) = (*next).into();
            next.update(*user)
        }

        #[cfg(test)]
        fn update_next(&self, next: &AccountId, user: &AccountId) -> (AccountId, u128, u128, u128, u128) {
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
        fn repay_any(&self, app: AccountId, user: AccountId, amount: u128, cash: u128, cash_owner: AccountId) -> Result<(AccountId, u128, u128, u128, u128, u128), LAssetError> {
            let mut app: contract_ref!(LAsset) = app.into();
            app.repay(user, amount, cash, cash_owner)
        }
        #[cfg(test)]
        fn repay_any(&self, app: AccountId, user: AccountId, amount: u128, cash: u128, cash_owner: AccountId) -> Result<(AccountId, u128, u128, u128, u128, u128), LAssetError> {
            unsafe {
                if app == AccountId::from([0x1; 32]) {
                    return L_BTC.as_mut().unwrap().repay(user, amount, cash, cash_owner);
                }
                if app == AccountId::from([0x2; 32]) {
                    return L_USDC.as_mut().unwrap().repay(user, amount, cash, cash_owner);
                }
                if app == AccountId::from([0x3; 32]) {
                    return L_ETH.as_mut().unwrap().repay(user, amount, cash, cash_owner);
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
        #[cfg(not(test))]
        fn approve_underlying(&self, token: AccountId, to: AccountId, value: u128) -> Result<(), PSP22Error> {
            let mut token: contract_ref!(PSP22) = token.into();
            token.approve(to, value)
        }
        #[cfg(test)]
        fn approve_underlying(&self, token: AccountId, to: AccountId, value: u128) -> Result<(), PSP22Error> {
            Ok(())
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

            let new_collaterals = if let Some(nc) = self.collaterals.checked_add(amount) {
                Ok(nc)
            } else {
                Err(LAssetError::DepositOverflow)
            }?;
            //Impossible to overflow, proofs/collateral.py for proof
            let new_collateral = add(collateral, amount);

            //it is crucial to update those two variables together
            self.collaterals = new_collaterals;
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

            let collateral = if let Some(c) = self.collateral.get(caller) {
                Ok(c)
            } else {
                Err(LAssetError::WithdrawWithoutDeposit)
            }?;

            let new_collateral = if let Some(nc) = collateral.checked_sub(amount) {
                Ok(nc)
            } else {
                Err(LAssetError::WithdrawOverflow)
            }?;
            let new_collaterals = sub(self.collaterals, amount);

            //We can ignore the fact, that user did not borrow anything yet, because
            //Borrow shares are not updated in this call
            let borrowed = self.borrowed.get(caller).unwrap_or(0);
            let borrowable = self.borrowable;
            let borrows = self.borrows;

            let accruer = logic::Accruer {
                now,
                updated_at,
                liquidity: self.liquidity,
                borrowable,
                standard_rate: self.standard_rate,
                emergency_rate: self.emergency_rate,
                standard_min_rate: self.standard_min_rate,
                emergency_max_rate: self.emergency_max_rate,
            };
            let liquidity = accruer.accrue();

            let quoter = logic::Quoter {
                price: self.price,
                price_scaler: self.price_scaler,
                borrowed,
                borrows,
                liquidity,
            };
            let quoted_collateral = quoter.quote(new_collateral);
            let quoted_debt = quoter.quote_debt(borrowable);
            let valuator = logic::Valuator {
                margin: self.initial_margin,
                haircut: self.initial_haircut,
                quoted_collateral,
                quoted_debt,
            };
            let (mut collateral_value, mut debt_value) = valuator.values();

            //Collateral must be updated before update
            //Inside update_all, we call next, so it is possible to reenter withdraw
            //Those values can be updated now, because update does not affect them
            self.collaterals = new_collaterals;
            self.collateral.insert(caller, &new_collateral);

            //We can update those now, because, updating other pools does not affect them
            //If we do it later, it would be possible to reenter and currupt total liquidity state
            self.updated_at = now;
            self.liquidity = liquidity;

            //inline update_all
            let mut next = self.next;
            while next != this {
                let (next2, next_collateral_value, next_debt_value, _, _) = self.update_next(&next, &caller);
                next = next2;
                collateral_value = collateral_value.saturating_add(next_collateral_value);
                debt_value = debt_value.saturating_add(next_debt_value);
            }
            if collateral_value < debt_value {
                Err(LAssetError::CollateralValueTooLow)
            } else {
                Ok(())
            }?;

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
            let borrowable = self.borrowable;
            let accruer = logic::Accruer {
                now,
                updated_at,
                liquidity: self.liquidity,
                borrowable,
                standard_rate: self.standard_rate,
                emergency_rate: self.emergency_rate,
                standard_min_rate: self.standard_min_rate,
                emergency_max_rate: self.emergency_max_rate,
            };
            let liquidity = accruer.accrue();

            let shares = self.shares;
            //First mint does not require any extra actions
            let share = self.share.get(caller).unwrap_or(0);

            let new_liquidity = {
                let r = liquidity.checked_add(amount);
                r.ok_or(LAssetError::MintLiquidityOverflow)
            }?;
            let minted = {
                let w = mulw(amount, shares);
                if let Some(m) = div_rate(w, liquidity) {
                    Ok(m)
                } else {
                    // First shares are scalled by 2^16. It limits total_shares to 2^112
                    if let Some(first) = amount.checked_shl(16) {
                        Ok(first)
                    } else {
                        Err(LAssetError::MintOverflow)
                    }
                }
            }?;
            
            //impossible to overflow IF total_liquidity is tracked correctly
            let new_shares = if let Some(ns) = shares.checked_add(minted) {
                Ok(ns)
            } else {
                Err(LAssetError::MintSharesOverflow)
            }?;
            let new_share = add(share, minted);
            let new_borrowable = add(borrowable, amount);

            //it is crucial to update those four variables together
            self.liquidity = new_liquidity;
            self.shares = new_shares;
            self.share.insert(caller, &new_share);
            self.borrowable = new_borrowable;

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

            let borrowable = self.borrowable;
            let accruer = logic::Accruer {
                now,
                updated_at,
                liquidity: self.liquidity,
                borrowable,
                standard_rate: self.standard_rate,
                emergency_rate: self.emergency_rate,
                standard_min_rate: self.standard_min_rate,
                emergency_max_rate: self.emergency_max_rate,
            };
            let liquidity = accruer.accrue();

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

            //TODO: resolve potential front running
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
            
            let current = self.env().account_id();
            
            let borrowable = self.borrowable;
            let accruer = logic::Accruer {
                now,
                updated_at,
                liquidity: self.liquidity,
                borrowable,
                standard_rate: self.standard_rate,
                emergency_rate: self.emergency_rate,
                standard_min_rate: self.standard_min_rate,
                emergency_max_rate: self.emergency_max_rate,
            };
            let liquidity = accruer.accrue();


            let new_borrowable = if let Some(r) = borrowable.checked_sub(amount) {
                Ok(r)
            } else {
                Err(LAssetError::BorrowableOverflow)
            }?;

            let borrows = self.borrows;
            let debt = sub(liquidity, borrowable);
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
                    if let Some(first) = amount.checked_shl(16) {
                        Ok(first)
                    } else {
                        Err(LAssetError::BorrowOverflow)
                    }
                }
            }?;

            let borrowed = if let Some(borrowed) = self.borrowed.get(caller) {
                Ok(borrowed)
            } else if self.env().transferred_value() != GAS_COLLATERAL {
                Err(LAssetError::FirstBorrowRequiresGasCollateral)
            } else {
                Ok(0)
            }?;

            let collateral = self.collateral.get(caller).unwrap_or(0);
            
            let new_borrows = if let Some(nb) = borrows.checked_add(minted) {
                Ok(nb)
            } else {
                Err(LAssetError::BorrowSharesOverflow)
            }?;
            let new_borrowed = add(borrowed, minted);
            
            let quoter = logic::Quoter {
                price: self.price,
                price_scaler: self.price_scaler,
                borrowed: new_borrowed,
                borrows: new_borrows,
                liquidity,
            };
            let quoted_collateral = quoter.quote(collateral);
            let quoted_debt = quoter.quote_debt(new_borrowable);

            let valuator = logic::Valuator {
                margin: self.initial_margin,
                haircut: self.initial_haircut,
                quoted_collateral,
                quoted_debt,
            };
            let (mut collateral_value, mut debt_value) = valuator.values();
            
            //it is crucial to update those three variables together
            self.borrowable = new_borrowable;
            self.borrows = new_borrows;
            self.borrowed.insert(caller, &new_borrowed);

            self.liquidity = liquidity;
            self.updated_at = now;

            let mut next = self.next;
            while next != current {
                let (next2, next_collateral_value, next_debt_value, _, _) = self.update_next(&next, &caller);
                next = next2;
                collateral_value = collateral_value.saturating_add(next_collateral_value);
                debt_value = debt_value.saturating_add(next_debt_value);
            }
            if collateral_value < debt_value {
                Err(LAssetError::CollateralValueTooLow)
            } else {
                Ok(())
            }?;

            if let Err(e) = self.transfer_underlying(caller, amount) {
                Err(LAssetError::BorrowTransferFailed(e))
            } else {
                Ok(())
            }?;

            Ok(())
        }

        

        #[ink(message)]
        pub fn liquidate(&mut self, user: AccountId, repay_asset: AccountId, repay_underlying: AccountId, amount: u128, cash: u128) -> Result<(), LAssetError> {
            let caller = self.env().caller();
            let this = self.env().account_id();
            if this == repay_asset {
                return Err(LAssetError::LiquidateSelf);
            }

            if let Err(e) = self.transfer_from_underlying(repay_underlying, caller, this, cash) {
                Err(LAssetError::LiquidateTransferFailed(e))
            } else {
                Ok(())
            }?;
            if let Err(e) = self.approve_underlying(repay_underlying, repay_asset, cash) {
                Err(LAssetError::LiquidateApproveFailed(e))
            } else {
                Ok(())
            }?;
            let (repay_next, repaid, mut initial_collateral_value, mut initial_debt_value, mut maintenance_collateral_value, mut maintenance_debt_value) = self.repay_any(repay_asset, user, amount, cash, caller)?;
            
            let updated_at = self.updated_at;
            let timestamp = self.env().block_timestamp();
            let now = logic::get_now(timestamp, updated_at);
            let borrowable = self.borrowable;

            let accruer = logic::Accruer {
                now,
                updated_at,
                liquidity: self.liquidity,
                borrowable,
                standard_rate: self.standard_rate,
                emergency_rate: self.emergency_rate,
                standard_min_rate: self.standard_min_rate,
                emergency_max_rate: self.emergency_max_rate,
            };
            let liquidity = accruer.accrue();
            let collateral = if let Some(c) = self.collateral.get(user) {
                Ok(c)
            } else {
                Err(LAssetError::LiquidateForNothing)
            }?;
            let borrowed = self.borrowed.get(user).unwrap_or(0);
            let quoter = logic::Quoter {
                price: self.price,
                price_scaler: self.price_scaler,
                borrowed,
                borrows: self.borrows,
                liquidity,
            };
            let delta_collateral = quoter.dequote(repaid);
            let new_collateral = if let Some(r) = collateral.checked_sub(delta_collateral) {
                Ok(r)
            } else {
                Err(LAssetError::LiquidateCollateralOverflow)
            }?;
            let new_collaterals = sub(self.collaterals, delta_collateral);

            self.collaterals = new_collaterals;
            self.collateral.insert(user, &new_collateral);

            self.liquidity = liquidity;
            self.updated_at = now;

            let quoted_old_collateral = quoter.quote(collateral);
            let quoted_collateral = quoter.quote(new_collateral);
            let borrowable = self.borrowable;
            let quoted_debt = quoter.quote(borrowable);
            
            let initial_valuator = logic::Valuator {
                margin: self.initial_margin,
                haircut: self.initial_haircut,
                quoted_collateral,
                quoted_debt,
            };
            let (initial_collateral_value_delta, initial_debt_value_delta) = initial_valuator.values();
            initial_collateral_value = initial_collateral_value.saturating_add(initial_collateral_value_delta);
            initial_debt_value = initial_debt_value.saturating_add(initial_debt_value_delta);

            let mainteneance_valuator = logic::Valuator {
                margin: self.maintenance_margin,
                haircut: self.maintenance_haircut,
                quoted_collateral: quoted_old_collateral,
                quoted_debt,
            };
            let (maintenance_collateral_value_delta, maintenance_debt_value_delta) = mainteneance_valuator.values();
            maintenance_collateral_value = maintenance_collateral_value.saturating_add(maintenance_collateral_value_delta);
            maintenance_debt_value = maintenance_debt_value.saturating_add(maintenance_debt_value_delta);

            let mut invalid = true;
            let mut next = self.next;
            while next != this {
                if next == repay_asset {
                    next = repay_next;
                    invalid = false;
                    continue;
                }
                let (next2, next_initial_collateral_value, next_initial_debt_value, next_maintenance_collateral_value, next_maintenance_debt_value) = self.update_next(&next, &user);
                next = next2;
                initial_collateral_value = initial_collateral_value.saturating_add(next_initial_collateral_value);
                initial_debt_value = initial_debt_value.saturating_add(next_initial_debt_value);
                maintenance_collateral_value = maintenance_collateral_value.saturating_add(next_maintenance_collateral_value);
                maintenance_debt_value = maintenance_debt_value.saturating_add(next_maintenance_debt_value);
            }
            if invalid {
                return Err(LAssetError::LiquidateInvalid);
            }
            if maintenance_collateral_value >= maintenance_debt_value {
                return Err(LAssetError::LiquidateTooEarly);
            }
            if initial_collateral_value >= initial_debt_value {
                return Err(LAssetError::LiquidateTooMuch);
            } 

            if let Err(e) = self.transfer_underlying(caller, delta_collateral) {
                Err(LAssetError::LiquidateTransferFailed(e))
            } else {
                Ok(())
            }

        }
    }

    impl LAsset for LAssetContract {
        #[ink(message)]
        fn repay(&mut self, user: AccountId, amount: u128, cash: u128, cash_owner: AccountId) -> Result<(AccountId, u128, u128, u128, u128, u128), LAssetError> {
            //You can repay for yourself only
            let caller = self.env().caller();
            let this = self.env().account_id();

            //Transfer first to avoid read only reentrancy attack
            if let Err(e) = self.transfer_from_underlying(self.underlying_token, caller, this, cash) {
                Err(LAssetError::RepayTransferFailed(e))
            } else {
                Ok(())
            }?;

            let updated_at = self.updated_at;
            let timestamp = self.env().block_timestamp();
            let now = logic::get_now(timestamp, updated_at);

            let borrowed = if let Some(r) = self.borrowed.get(user) {
                Ok(r)
            } else {
                Err(LAssetError::RepayWithoutBorrow)
            }?;

            let (amount, new_borrowed) = if let Some(r) = borrowed.checked_sub(amount) {
                Ok((amount, r))
            } else {
                Ok((borrowed, 0))
            }?;

            let borrowable = self.borrowable;
            let accruer = logic::Accruer {
                now,
                updated_at,
                liquidity: self.liquidity,
                borrowable,
                standard_rate: self.standard_rate,
                emergency_rate: self.emergency_rate,
                standard_min_rate: self.standard_min_rate,
                emergency_max_rate: self.emergency_max_rate,
            };
            let liquidity = accruer.accrue();
            
            let new_debt = sub(liquidity, borrowable);
            let borrows = self.borrows;
            let repaid = {
                let w = mulw(amount, new_debt);
                ceil_rate(w, borrows).unwrap_or(0)
            };
            let extra_cash = if let Some(r) = cash.checked_sub(repaid) {
                Ok(r)
            } else {
                Err(LAssetError::RepayInsufficientCash)
            }?;

            let cash = self.cash.get(cash_owner).unwrap_or(0);
            let new_cash = if let Some(r) = cash.checked_add(extra_cash) {
                Ok(r)
            } else {
                Err(LAssetError::RepayCashOverflow)
            }?;
            let new_borrowable = add(borrowable, repaid);
            let new_borrows = sub(borrows, amount);

            self.cash.insert(cash_owner, &new_cash);
            self.borrowable = new_borrowable;
            self.borrows = new_borrows;
            self.borrowed.insert(user, &new_borrowed);

            self.liquidity = liquidity;
            self.updated_at = now;

            let qouter = logic::Quoter {
                price: self.price,
                price_scaler: self.price_scaler,
                borrowed: new_borrowed,
                borrows: new_borrows,
                liquidity,
            };
            let collateral = self.collateral.get(user).unwrap_or(0);
            
            let quoted_collateral = qouter.quote(collateral);
            let quoted_debt = qouter.quote_debt(new_borrowable);
            let initial_valuator = logic::Valuator {
                margin: self.initial_margin,
                haircut: self.initial_haircut,
                quoted_collateral,
                quoted_debt,
            };
            let (initial_collateral_value, initial_debt_value) = initial_valuator.values();
            
            let quoted_old_debt = qouter.quote_debt(borrowable);
            let maintenance_valuator = logic::Valuator {
                margin: self.maintenance_margin,
                haircut: self.maintenance_haircut,
                quoted_collateral,
                quoted_debt: quoted_old_debt,
            };
            let (maintenance_collateral_value, maintenance_debt_value) = maintenance_valuator.values();

            let qouted_repaid = qouter.quote(repaid);
            let next = self.next;
            
            Ok((next, qouted_repaid, initial_collateral_value, initial_debt_value, maintenance_collateral_value, maintenance_debt_value))
        }

        #[ink(message)]
        fn update(&mut self, user: AccountId) -> (AccountId, u128, u128, u128, u128) {
            let updated_at = self.updated_at;
            let now = self.get_now(updated_at);
            let collateral = self.collateral.get(user).unwrap_or(0);
            let borrowed = self.borrowed.get(user).unwrap_or(0);
            let borrows = self.borrows;
            let borrowable = self.borrowable;
            
            let accruer = logic::Accruer {
                now,
                updated_at,
                liquidity: self.liquidity,
                borrowable,
                standard_rate: self.standard_rate,
                emergency_rate: self.emergency_rate,
                standard_min_rate: self.standard_min_rate,
                emergency_max_rate: self.emergency_max_rate,
            };
            let liquidity = accruer.accrue();

            self.updated_at = now;
            self.liquidity = liquidity;

            let quoter = logic::Quoter {
                price: self.price,
                price_scaler: self.price_scaler,
                borrowed,
                borrows,
                liquidity,
            };
            let quoted_collateral = quoter.quote(collateral);
            let quoted_debt = quoter.quote_debt(borrowable);
            let valuator = logic::Valuator {
                margin: self.initial_margin,
                haircut: self.initial_haircut,
                quoted_collateral,
                quoted_debt,
            };
            let (collateral_value, debt_value) = valuator.values();
            let maintenance_valuator = logic::Valuator {
                margin: self.maintenance_margin,
                haircut: self.maintenance_haircut,
                quoted_collateral,
                quoted_debt,
            };
            let (maintenance_collateral_value, maintenance_debt_value) = maintenance_valuator.values();

            let next = self.next;
            (next, collateral_value, debt_value, maintenance_collateral_value, maintenance_debt_value)
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

    impl PSP22Metadata for LAssetContract {
        #[ink(message)]
        fn token_name(&self) -> Option<String> {
            self.name.clone()
        }

        #[ink(message)]
        fn token_symbol(&self) -> Option<String> {
            self.symbol.clone()
        }

        #[ink(message)]
        fn token_decimals(&self) -> u8 {
            self.decimals
        }
    } 

    #[cfg(not(test))]
    /// If the asset is not compatible with PSP22Metadata, the decimals will be set to 6
    fn fetch_psp22_metadata(token: AccountId) -> (Option<String>, Option<String>, u8) {
        use ink::codegen::TraitCallBuilder;
        use ink::prelude::string::String;
        let token: contract_ref!(PSP22Metadata) = token.into();
        let name = token.call().token_name().transferred_value(0).try_invoke().unwrap_or(Ok(None)).unwrap_or(None);
        let symbol = token.call().token_symbol().transferred_value(0).try_invoke().unwrap_or(Ok(None)).unwrap_or(None);
        let decimals = token.call().token_decimals().transferred_value(0).try_invoke().unwrap_or(Ok(DEFAULT_DECIMALS)).unwrap_or(DEFAULT_DECIMALS);

        let l_name = match name {
            Some(n) => Some(String::from("L-") + &n),
            None => None
        };
        let l_symbol = match symbol {
            Some(s) => Some(String::from("L-") + &s),
            None => None
        };

        (l_name, l_symbol, decimals)
    }

    #[cfg(test)]
    fn fetch_psp22_metadata(token: AccountId) -> (Option<String>, Option<String>, u8) {
        (Some("L-TestToken".to_string()), Some("L-TT".to_string()), 16)
    }

}
