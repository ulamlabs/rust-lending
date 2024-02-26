#![cfg_attr(not(feature = "std"), no_std, no_main)]

use traits::errors::LAssetError;
use ink::primitives::AccountId;

mod logic;

#[cfg(test)]
mod tests;

#[ink::trait_definition]
pub trait LAsset {
    #[ink(message)]
    fn update(&mut self, user: AccountId) -> (AccountId, u128, u128);

    #[ink(message)]
    fn repay_or_update(&mut self, user: AccountId, amount: u128, cash: u128, cash_owner: AccountId) -> Result<(AccountId, u128, u128, u128, u128, u128), LAssetError>;
}


#[ink::contract]
mod finance2 {
    #[cfg(not(test))]
    use ink::contract_ref;
    #[cfg(not(test))]
    use ink::prelude::vec;
    use ink::prelude::vec::Vec;
    use ink::prelude::string::String;
    use traits::FlashLoanPool;
    use traits::errors::LAssetError;
    use traits::psp22::{PSP22Error, PSP22Metadata, Transfer, Approval, PSP22};
    use crate::logic::{self, add, mulw, sub};

    const GAS_COLLATERAL: u128 = 1_000_000; // TODO find something less random
    const DEFAULT_DECIMALS: u8 = 6;

    use ink::storage::Mapping;
    use crate::LAsset;

    #[ink(storage)]
    pub struct LAssetContract {
        admin: AccountId,
        underlying_token: AccountId,
        last_updated_at: Timestamp,

        next: AccountId,

        total_collateral: u128,
        collateral: Mapping<AccountId, u128>,
    
        last_total_liquidity: u128,
        total_borrowable: u128,
    
        total_shares: u128,
        shares: Mapping<AccountId, u128>,
        allowance: Mapping<(AccountId, AccountId), u128>,
    
        total_bonds: u128,
        bonds: Mapping<AccountId, u128>,

        standard_rate: u128,
        standard_min_rate: u128,

        emergency_rate: u128,
        emergency_max_rate: u128,

        initial_margin: u128,
        maintenance_margin: u128,

        initial_haircut: u128,
        maintenance_haircut: u128,

        discount: u128,

        price: u128,
        price_scaler: u128,

        cash: Mapping<AccountId, u128>,
        whitelist: Mapping<AccountId, AccountId>,

        // PSP22Metadata
        name: Option<String>,
        symbol: Option<String>,
        decimals: u8,

        // flash loan contract address
        pub flash: AccountId,
    }

    


    impl LAssetContract {
        #[allow(clippy::too_many_arguments)]
        #[ink(constructor)]
        pub fn new(
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
            discount: u128,
            price_scaler: u128,
            flash: AccountId,
        ) -> Self {
            let (name, symbol, decimals) = fetch_psp22_metadata(underlying_token);
            let admin: AccountId = Self::env().caller();

            Self { 
                admin,
                underlying_token,
                last_updated_at: Self::env().block_timestamp(),
                next,
                total_collateral: 0,
                collateral: Mapping::new(),
                last_total_liquidity: 0,
                total_shares: 0,
                shares: Mapping::new(),
                allowance: Mapping::new(),
                total_borrowable: 0,
                total_bonds: 0,
                bonds: Mapping::new(),
                standard_rate,
                standard_min_rate,
                emergency_rate,
                emergency_max_rate,
                initial_margin,
                maintenance_margin,
                initial_haircut,
                maintenance_haircut,
                discount,
                price: 0,
                price_scaler,
                cash: Mapping::new(),
                whitelist: Mapping::new(),
                name,
                symbol,
                decimals,
                flash,
             }
        }
        #[ink(message)]
        pub fn set_price(&mut self, price: u128) -> Result<(), LAssetError> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(LAssetError::CallerIsNotAdmin);
            }
            self.price = price;

            Ok(())
        }

        #[cfg(not(test))]
        fn transfer_underlying(&self, to: AccountId, value: u128) -> Result<(), PSP22Error> {
            let mut token: contract_ref!(PSP22) = self.underlying_token.into();
            token.transfer(to, value, vec![])
        }
        #[cfg(test)]
        #[allow(unused_variables)]
        fn transfer_underlying(&self, to: AccountId, value: u128) -> Result<(), PSP22Error> {
            Ok(())
        }
        
        //There function does not require anything
        //Depositing collateral is absolutely independent
        //The only risk is that use will deposit small amount of tokens
        //And it's going to be hard to liquidate such user
        //We have to introduce some kind of gas collateral
        #[ink(message)]
        pub fn deposit(&mut self, amount: u128) -> Result<(), LAssetError> {
            let caller = self.env().caller();
            let this = self.env().account_id();

            transfer_from(self.underlying_token, caller, this, GAS_COLLATERAL).map_err(LAssetError::DepositTransferFailed)?;

            //It is important to check if user collateral cannot be initialized in any other way
            //It would allow user to deposit without gas collateral
            let collateral = if let Some(c) = self.collateral.get(caller) {
                Ok(c)
            } else if self.env().transferred_value() != GAS_COLLATERAL {
                Err(LAssetError::FirstDepositRequiresGasCollateral)
            } else {
                Ok(0)
            }?;
            
            let new_total_collateral = self.total_collateral.checked_add(amount).ok_or(LAssetError::DepositOverflow)?;
            //Impossible to overflow, proofs/collateral.py for proof
            let new_collateral = add(collateral, amount);

            //it is crucial to update those two variables together
            self.total_collateral = new_total_collateral;
            self.collateral.insert(caller, &new_collateral);

            Ok(())
        }

        /// This function is very dangerous, because collateral is the only thing
        /// That keep borrower from running away with bonds liquidity
        /// It is crucial to check if collateral value is greater than value of bonds liquidity
        #[ink(message)]
        pub fn withdraw(&mut self, amount: u128) -> Result<(), LAssetError> {
            let caller = self.env().caller();

            let collateral = self.collateral.get(caller).ok_or(LAssetError::WithdrawWithoutDeposit)?;
            let new_collateral = collateral.checked_sub(amount).ok_or(LAssetError::WithdrawOverflow)?;
            let new_total_collateral = sub(self.total_collateral, amount);

            //TODO: having debt and collateral at the same time is not effective
            let bonds = self.bonds.get(caller).unwrap_or(0);
            let total_borrowable = self.total_borrowable;
            let total_bonds = self.total_bonds;

            let accruer = logic::Accruer {
                now: self.env().block_timestamp(),
                updated_at: self.last_updated_at,
                total_liquidity: self.last_total_liquidity,
                total_borrowable,
                standard_rate: self.standard_rate,
                emergency_rate: self.emergency_rate,
                standard_min_rate: self.standard_min_rate,
                emergency_max_rate: self.emergency_max_rate,
            };
            let (total_liquidity, updated_at) = accruer.accrue();

            let quoter = logic::Quoter {
                price: self.price,
                price_scaler: self.price_scaler,
                bonds,
                total_bonds,
                total_liquidity,
            };
            let quoted_collateral = quoter.quote(new_collateral);
            let quoted_debt = quoter.quote_debt(total_borrowable);
            let valuator = logic::Valuator {
                margin: self.initial_margin,
                haircut: self.initial_haircut,
                quoted_collateral,
                quoted_debt,
            };
            let (mut total_icv, mut total_idv) = valuator.values();

            self.total_collateral = new_total_collateral;
            self.collateral.insert(caller, &new_collateral);

            self.last_total_liquidity = total_liquidity;
            self.last_updated_at = updated_at;

            let mut current = self.next;
            let this = self.env().account_id();
            while current != this {
                let (next, icv, idv) = update_next(&current, &caller);
                current = next;
                total_icv = total_icv.saturating_add(icv);
                total_idv = total_idv.saturating_add(idv);
            }
            logic::gte(total_icv, total_idv, LAssetError::CollateralValueTooLow)?;

            transfer(self.underlying_token, caller, GAS_COLLATERAL).map_err(LAssetError::WithdrawTransferFailed)
        }

        /// Specify an amount of underlying tokens to deposit and receive pool shares.
        /// Number of minted shares depends on total liquidity and total shares.
        #[ink(message)]
        pub fn mint(&mut self, amount: u128) -> Result<(), LAssetError> {
            let caller = self.env().caller();
            let this = self.env().account_id();

            transfer_from(self.underlying_token, caller, this, amount).map_err(LAssetError::MintTransferFailed)?;

            let total_borrowable = self.total_borrowable;
            let accruer = logic::Accruer {
                now: self.env().block_timestamp(),
                updated_at: self.last_updated_at,
                total_liquidity: self.last_total_liquidity,
                total_borrowable,
                standard_rate: self.standard_rate,
                emergency_rate: self.emergency_rate,
                standard_min_rate: self.standard_min_rate,
                emergency_max_rate: self.emergency_max_rate,
            };
            let (total_liquidity, updated_at) = accruer.accrue();

            let total_shares = self.total_shares;
            let shares: u128 = self.shares.get(caller).unwrap_or(0);
            
            let new_total_liquidity = total_liquidity.checked_add(amount).ok_or(LAssetError::MintLiquidityOverflow)?;
            let minted = mulw(amount, total_shares).div_rate(total_liquidity).unwrap_or(amount);
            
            let new_total_shares = add(total_shares, minted);
            let new_share = add(shares, minted);
            let new_borrowable = add(total_borrowable, amount);

            self.total_shares = new_total_shares;
            self.shares.insert(caller, &new_share);
            
            self.total_borrowable = new_borrowable;
            self.last_total_liquidity = new_total_liquidity;
            self.last_updated_at = updated_at;
            
            self.env().emit_event(Transfer {from: None, to: Some(caller), value: minted});

            Ok(())
        }

        /// Burn a specified amount of shares and receive the underlying tokens
        #[ink(message)]
        pub fn burn(&mut self, amount: u128) -> Result<(), LAssetError> {
            let caller = self.env().caller();

            let total_borrowable = self.total_borrowable;
            let accruer = logic::Accruer {
                now: self.env().block_timestamp(),
                updated_at: self.last_updated_at,
                total_liquidity: self.last_total_liquidity,
                total_borrowable,
                standard_rate: self.standard_rate,
                emergency_rate: self.emergency_rate,
                standard_min_rate: self.standard_min_rate,
                emergency_max_rate: self.emergency_max_rate,
            };
            let (total_liquidity, updated_at) = accruer.accrue();

            let total_shares = self.total_shares;
            let shares = self.shares.get(caller).unwrap_or(0);

            let new_shares = shares.checked_sub(amount).ok_or(LAssetError::BurnOverflow)?;

            let to_withdraw = mulw(amount, total_liquidity).div_rate(total_shares).unwrap_or(0);
            let new_total_borrowable: u128 = total_borrowable.checked_sub(to_withdraw).ok_or(LAssetError::BurnTooMuch)?;

            let new_total_shares = sub(total_shares, amount);
            let new_total_liquidity = sub(total_liquidity, to_withdraw);

            self.total_shares = new_total_shares;
            self.shares.insert(caller, &new_shares);
            
            self.total_borrowable = new_total_borrowable;
            self.last_total_liquidity = new_total_liquidity;
            self.last_updated_at = updated_at;

            self.env().emit_event(Transfer {from: Some(caller), to: None, value: amount});

            transfer(self.underlying_token, caller, to_withdraw).map_err(LAssetError::BurnTransferFailed)
        }

        //In this function amount is amount of liquidity, not shares
        #[ink(message)]
        pub fn borrow(&mut self, amount: u128) -> Result<(), LAssetError> {
            let caller = self.env().caller();
            let this = self.env().account_id();

            let total_borrowable = self.total_borrowable;
            let accruer = logic::Accruer {
                now: self.env().block_timestamp(),
                updated_at: self.last_updated_at,
                total_liquidity: self.last_total_liquidity,
                total_borrowable,
                standard_rate: self.standard_rate,
                emergency_rate: self.emergency_rate,
                standard_min_rate: self.standard_min_rate,
                emergency_max_rate: self.emergency_max_rate,
            };
            let (total_liquidity, updated_at) = accruer.accrue();

            let new_total_borrowable = total_borrowable.checked_sub(amount).ok_or(LAssetError::BorrowableOverflow)?;

            let debt = sub(total_liquidity, total_borrowable);
            let total_bonds = self.total_bonds;
            let minted = mulw(amount, total_bonds).ceil_rate(debt).unwrap_or(amount);

            let bonds = if let Some(bonds) = self.bonds.get(caller) {
                Ok(bonds)
            } else if self.env().transferred_value() != GAS_COLLATERAL {
                Err(LAssetError::FirstBorrowRequiresGasCollateral)
            } else {
                Ok(0)
            }?;

            //TODO: having debt and collateral at the same time is not effective
            let collateral = self.collateral.get(caller).unwrap_or(0);
            
            let new_total_bonds = add(total_bonds, minted);
            let new_bonds = add(bonds, minted);
            
            let quoter = logic::Quoter {
                price: self.price,
                price_scaler: self.price_scaler,
                bonds: new_bonds,
                total_bonds: new_total_bonds,
                total_liquidity,
            };
            let quoted_collateral = quoter.quote(collateral);
            let quoted_debt = quoter.quote_debt(new_total_borrowable);

            let valuator = logic::Valuator {
                margin: self.initial_margin,
                haircut: self.initial_haircut,
                quoted_collateral,
                quoted_debt,
            };
            let (mut total_icv, mut total_idv) = valuator.values();
            
            self.total_bonds = new_total_bonds;
            self.bonds.insert(caller, &new_bonds);
            
            self.total_borrowable = new_total_borrowable;
            self.last_total_liquidity = total_liquidity;
            self.last_updated_at = updated_at;

            let mut current = self.next;
            while current != this {
                let (next, icv, idv) = update_next(&current, &caller);
                current = next;
                total_icv = total_icv.saturating_add(icv);
                total_idv = total_idv.saturating_add(idv);
            }
            logic::gte(total_icv, total_idv, LAssetError::CollateralValueTooLow)?;

            transfer(self.underlying_token, caller, amount).map_err(LAssetError::BorrowTransferFailed)
        }

        #[ink(message)]
        pub fn increase_cash(&mut self, spender: AccountId, amount: u128) -> Result<(), LAssetError> {
            let caller = self.env().caller();
            let this = self.env().account_id();
            transfer_from(self.underlying_token, caller, this, amount).map_err(LAssetError::IncreaseCashTransferFailed)?;
            
            let cash = self.cash.get(caller).unwrap_or(0);
            let new_cash = cash.checked_add(amount).ok_or(LAssetError::IncreaseCashOverflow)?;

            self.cash.insert(caller, &new_cash);
            self.whitelist.insert(caller, &spender);

            Ok(())
        }

        #[ink(message)]
        pub fn liquidate(&mut self, user: AccountId, amount: u128, cash: u128) -> Result<(), LAssetError> {
            let caller = self.env().caller();
            let this = self.env().account_id();

            let mut total_icv: u128 = 0;
            let mut total_idv: u128 = 0;
            let mut total_mcv: u128 = 0;
            let mut total_mdv: u128 = 0;
            let mut total_repaid: u128 = 0;

            let mut current = self.next;
            while current != this {
                let (next, repaid, icv, idv, mcv, mdv) = repay_or_update(current, user, amount, cash, caller)?;
                
                current = next;
                total_repaid = repaid.saturating_add(repaid);
                total_icv = total_icv.saturating_add(icv);
                total_idv = total_idv.saturating_add(idv);
                total_mcv = total_mcv.saturating_add(mcv);
                total_mdv = total_mdv.saturating_add(mdv);
            }

            let total_borrowable = self.total_borrowable;

            let accruer = logic::Accruer {
                now: self.env().block_timestamp(),
                updated_at: self.last_updated_at,
                total_liquidity: self.last_total_liquidity,
                total_borrowable,
                standard_rate: self.standard_rate,
                emergency_rate: self.emergency_rate,
                standard_min_rate: self.standard_min_rate,
                emergency_max_rate: self.emergency_max_rate,
            };
            let (total_liquidity, updated_at) = accruer.accrue();
            let collateral = self.collateral.get(user).ok_or(LAssetError::LiquidateForNothing)?;

            let bonds = self.bonds.get(user).unwrap_or(0);
            let quoter = logic::Quoter {
                price: self.price,
                price_scaler: self.price_scaler,
                bonds,
                total_bonds: self.total_bonds,
                total_liquidity,
            };
            let collateral_to_withdraw = quoter.dequote(self.discount, total_repaid);
            let new_collateral = collateral.checked_sub(collateral_to_withdraw).ok_or(LAssetError::LiquidateCollateralOverflow)?;
            let new_total_collateral = sub(self.total_collateral, collateral_to_withdraw);

            self.total_collateral = new_total_collateral;
            self.collateral.insert(user, &new_collateral);

            self.last_total_liquidity = total_liquidity;
            self.last_updated_at = updated_at;

            let quoted_old_collateral = quoter.quote(collateral);
            let quoted_collateral = quoter.quote(new_collateral);
            let total_borrowable = self.total_borrowable;
            let quoted_debt = quoter.quote(total_borrowable);
            
            let initial_valuator = logic::Valuator {
                margin: self.initial_margin,
                haircut: self.initial_haircut,
                quoted_collateral,
                quoted_debt,
            };
            let (icv, idv) = initial_valuator.values();
            total_icv = total_icv.saturating_add(icv);
            total_idv = total_idv.saturating_add(idv);

            let mainteneance_valuator = logic::Valuator {
                margin: self.maintenance_margin,
                haircut: self.maintenance_haircut,
                quoted_collateral: quoted_old_collateral,
                quoted_debt,
            };
            let (mcv, mdv) = mainteneance_valuator.values();
            total_mcv = total_mcv.saturating_add(mcv);
            total_mdv = total_mdv.saturating_add(mdv);


            logic::lt(total_mdv, total_mcv, LAssetError::LiquidateTooEarly)?;
            logic::lt(total_idv, total_icv, LAssetError::LiquidateTooMuch)?;

            transfer(self.underlying_token, caller, collateral_to_withdraw).map_err(LAssetError::LiquidateTransferFailed)
        }

        fn inner_repay(&mut self, caller: AccountId, user: AccountId, amount: u128, cash: u128, cash_owner: AccountId, total_borrowable: u128
        ) -> Result<(u128, u128, u128, u128, u128), LAssetError> {
            let bonds = self.bonds.get(user).ok_or(LAssetError::RepayWithoutBorrow)?;

            let (amount, new_borrowed) = if let Some(r) = bonds.checked_sub(amount) {
                Ok((amount, r))
            } else {
                Ok((bonds, 0))
            }?;

            let accruer = logic::Accruer {
                now: self.env().block_timestamp(),
                updated_at: self.last_updated_at,
                total_liquidity: self.last_total_liquidity,
                total_borrowable,
                standard_rate: self.standard_rate,
                emergency_rate: self.emergency_rate,
                standard_min_rate: self.standard_min_rate,
                emergency_max_rate: self.emergency_max_rate,
            };
            let (total_liquidity, updated_at) = accruer.accrue();
            
            let new_debt = sub(total_liquidity, total_borrowable);
            let total_bonds = self.total_bonds;
            let repaid = mulw(amount, new_debt).ceil_rate(total_bonds).unwrap_or(0);
            
            let extra_cash = cash.checked_sub(repaid).ok_or(LAssetError::RepayInsufficientCash)?;

            let cash = self.cash.get(cash_owner).unwrap_or(0);
            let new_cash = cash.checked_add(extra_cash).ok_or(LAssetError::RepayCashOverflow)?;

            let new_borrowable = add(total_borrowable, repaid);
            let new_total_bonds = sub(total_bonds, amount);

            self.total_bonds = new_total_bonds;
            self.bonds.insert(user, &new_borrowed);
            
            self.cash.insert(caller, &new_cash);
            
            self.total_borrowable = new_borrowable;
            self.last_total_liquidity = total_liquidity;
            self.last_updated_at = updated_at;

            Ok((repaid, new_borrowable, new_total_bonds, new_borrowed, total_liquidity))
        }


        #[ink(message)]
        pub fn repay(&mut self, user: AccountId, amount: u128, cash: u128) -> Result<(), LAssetError> {
            let caller = self.env().caller();
            let this = self.env().account_id();

            transfer_from(self.underlying_token, caller, this, amount).map_err(LAssetError::RepayTransferFailed)?;
            self.inner_repay(caller, user, amount, cash, caller, self.total_borrowable)?;

            Ok(())
        }
    }

    impl LAsset for LAssetContract {
        #[ink(message)]
        fn repay_or_update(&mut self, user: AccountId, amount: u128, cash: u128, cash_owner: AccountId) -> Result<(AccountId, u128, u128, u128, u128, u128), LAssetError> {
            let caller = self.env().caller();

            let valid_caller = self.whitelist.get(cash_owner).ok_or(LAssetError::RepayNotWhitelisted)?;
            let total_borrowable = self.total_borrowable;
            let (repaid, new_borrowable, new_total_bonds, new_bonds, total_liquidity) = if  caller == valid_caller {
                let old_cash = self.cash.get(cash_owner).unwrap_or(0);
                let new_cash = old_cash.checked_sub(cash).ok_or(LAssetError::RepayInsufficientCash)?;

                self.whitelist.remove(caller);
                self.cash.insert(caller, &new_cash);

                self.inner_repay(cash_owner, user, amount, cash, cash_owner, total_borrowable)?
            } else {
                let accurer = logic::Accruer {
                    now: self.last_updated_at,
                    updated_at: self.last_updated_at,
                    total_liquidity: self.last_total_liquidity,
                    total_borrowable,
                    standard_rate: self.standard_rate,
                    emergency_rate: self.emergency_rate,
                    standard_min_rate: self.standard_min_rate,
                    emergency_max_rate: self.emergency_max_rate,
                };
                let (total_liquidity, updated_at) = accurer.accrue();
                let bonds = self.bonds.get(user).unwrap_or(0);

                self.last_total_liquidity = total_liquidity;
                self.last_updated_at = updated_at;
                (0, total_borrowable, self.total_bonds, bonds, total_liquidity)
            };

            let qouter = logic::Quoter {
                price: self.price,
                price_scaler: self.price_scaler,
                bonds: new_bonds,
                total_bonds: new_total_bonds,
                total_liquidity,
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
            let (icv, idv) = initial_valuator.values();
            
            let quoted_old_debt = qouter.quote_debt(total_borrowable);
            let maintenance_valuator = logic::Valuator {
                margin: self.maintenance_margin,
                haircut: self.maintenance_haircut,
                quoted_collateral,
                quoted_debt: quoted_old_debt,
            };
            let (maintenance_collateral_value, maintenance_debt_value) = maintenance_valuator.values();

            let qouted_repaid = qouter.quote(repaid);
            
            Ok((self.next, qouted_repaid, icv, idv, maintenance_collateral_value, maintenance_debt_value))
        }

        #[ink(message)]
        fn update(&mut self, user: AccountId) -> (AccountId, u128, u128) {
            let collateral = self.collateral.get(user).unwrap_or(0);
            let bonds = self.bonds.get(user).unwrap_or(0);
            let total_bonds = self.total_bonds;
            let total_borrowable = self.total_borrowable;
            
            let accruer = logic::Accruer {
                now: self.env().block_timestamp(),
                updated_at: self.last_updated_at,
                total_liquidity: self.last_total_liquidity,
                total_borrowable,
                standard_rate: self.standard_rate,
                emergency_rate: self.emergency_rate,
                standard_min_rate: self.standard_min_rate,
                emergency_max_rate: self.emergency_max_rate,
            };
            let (total_liquidity, updated_at) = accruer.accrue();

            self.last_total_liquidity = total_liquidity;
            self.last_updated_at = updated_at;

            let quoter = logic::Quoter {
                price: self.price,
                price_scaler: self.price_scaler,
                bonds,
                total_bonds,
                total_liquidity,
            };
            let quoted_collateral = quoter.quote(collateral);
            let quoted_debt = quoter.quote_debt(total_borrowable);
            let valuator = logic::Valuator {
                margin: self.initial_margin,
                haircut: self.initial_haircut,
                quoted_collateral,
                quoted_debt,
            };
            let (icv, idv) = valuator.values();

            (self.next, icv, idv)
        }
    }

    impl FlashLoanPool for LAssetContract {
        #[ink(message)]
        fn take_cash(&mut self, amount: u128, target: AccountId) -> Result<AccountId, LAssetError> {
            let caller = self.env().caller();
            if caller != self.flash {
                return Err(LAssetError::FlashContractOnly);
            }
            self.transfer_underlying(target, amount).map_err(LAssetError::TakeCashFailed)?;

            Ok(self.underlying_token)
        }

        #[ink(message)]
        fn underlying_token(&self) -> AccountId {
            self.underlying_token
        }
    }

    impl PSP22 for LAssetContract {
        #[ink(message)]
        fn total_supply(&self) -> u128 {
            self.total_shares
        }

        #[ink(message)]
        fn balance_of(&self, owner: AccountId) -> u128 {
            self.shares.get(owner).unwrap_or(0)
        }

        #[ink(message)]
        fn allowance(&self, owner: AccountId, spender: AccountId) -> u128 {
            self.allowance.get((owner, spender)).unwrap_or(0)
        }

        #[ink(message)]
        fn transfer(&mut self, to: AccountId, value: u128, _data: Vec<u8>) -> Result<(), PSP22Error> {
            let from = self.env().caller();
            let from_shares = self.shares.get(from).unwrap_or(0);
            let to_shares = self.shares.get(to).unwrap_or(0);

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

            self.shares.insert(from, &new_from_shares);
            self.shares.insert(to, &new_to_shares);

            self.env().emit_event(event);
            Ok(())
        }

        #[ink(message)]
        fn transfer_from(&mut self, from: AccountId, to: AccountId, value: u128, _data: Vec<u8>) -> Result<(), PSP22Error> {
            let from_shares = self.shares.get(from).unwrap_or(0);
            let to_shares = self.shares.get(to).unwrap_or(0);
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

            self.shares.insert(from, &new_from_shares);
            self.shares.insert(to, &new_to_shares);
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
        let token: contract_ref!(PSP22Metadata) = token.into();
        let name = token.call().token_name().transferred_value(0).try_invoke().unwrap_or(Ok(None)).unwrap_or(None);
        let symbol = token.call().token_symbol().transferred_value(0).try_invoke().unwrap_or(Ok(None)).unwrap_or(None);
        let decimals = token.call().token_decimals().transferred_value(0).try_invoke().unwrap_or(Ok(DEFAULT_DECIMALS)).unwrap_or(DEFAULT_DECIMALS);

        let l_name = name.map(|n| {
            let mut name = String::from("L-");
            name.push_str(n.as_str());
            name
        });
        let l_symbol = symbol.map(|s| {
            let mut symbol = String::from("L-");
            symbol.push_str(s.as_str());
            symbol
        });

        (l_name, l_symbol, decimals)
    }

    #[cfg(test)]
    #[allow(unused_variables)]
    fn fetch_psp22_metadata(token: AccountId) -> (Option<String>, Option<String>, u8) {
        (Some("L-TestToken".to_string()), Some("L-TT".to_string()), 16)
    }

    #[cfg(test)]
    static mut L_BTC: Option<LAssetContract> = None;
    #[cfg(test)]
    static mut L_USDC: Option<LAssetContract> = None;
    #[cfg(test)]
    static mut L_ETH: Option<LAssetContract> = None;

    #[cfg(not(test))]
    fn update_next(next: &AccountId, user: &AccountId) -> (AccountId, u128, u128) {
        let mut next: contract_ref!(LAsset) = (*next).into();
        next.update(*user)
    }

    #[cfg(test)]
    fn update_next(next: &AccountId, user: &AccountId) -> (AccountId, u128, u128) {
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
    fn repay_or_update(app: AccountId, user: AccountId, amount: u128, cash: u128, cash_owner: AccountId) -> Result<(AccountId, u128, u128, u128, u128, u128), LAssetError> {
        let mut app: contract_ref!(LAsset) = app.into();
        app.repay_or_update(user, amount, cash, cash_owner)
    }
    #[cfg(test)]
    fn repay_or_update(app: AccountId, user: AccountId, amount: u128, cash: u128, cash_owner: AccountId) -> Result<(AccountId, u128, u128, u128, u128, u128), LAssetError> {
        unsafe {
            if app == AccountId::from([0x1; 32]) {
                return L_BTC.as_mut().unwrap().repay_or_update(user, amount, cash, cash_owner);
            }
            if app == AccountId::from([0x2; 32]) {
                return L_USDC.as_mut().unwrap().repay_or_update(user, amount, cash, cash_owner);
            }
            if app == AccountId::from([0x3; 32]) {
                return L_ETH.as_mut().unwrap().repay_or_update(user, amount, cash, cash_owner);
            }
            unreachable!();
        }
    }

    #[cfg(not(test))]
    fn transfer_from(token: AccountId, from: AccountId, to: AccountId, value: u128) -> Result<(), PSP22Error> {
        let mut token: contract_ref!(PSP22) = token.into();
        token.transfer_from(from, to, value, vec![])
    }
    #[cfg(test)]
    #[allow(unused_variables)]
    fn transfer_from(token: AccountId, from: AccountId, to: AccountId, value: u128) -> Result<(), PSP22Error> {
        Ok(())
    }

    #[cfg(not(test))]
    fn transfer(token: AccountId, to: AccountId, value: u128) -> Result<(), PSP22Error> {
        let mut token: contract_ref!(PSP22) = token.into();
        token.transfer(to, value, vec![])
    }
    #[cfg(test)]
    #[allow(unused_variables)]
    fn transfer(token: AccountId, to: AccountId, value: u128) -> Result<(), PSP22Error> {
        Ok(())
    }
}
