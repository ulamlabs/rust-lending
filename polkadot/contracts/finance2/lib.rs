#![cfg_attr(not(feature = "std"), no_std, no_main)]

mod logic;
mod errors;

#[ink::trait_definition]
pub trait LAsset {
    #[ink(message)]
    fn update(
        &mut self, 
        user: ink::primitives::AccountId
    ) -> (ink::primitives::AccountId, u128, u128);

    #[ink(message)]
    fn repay_or_update(
        &mut self, 
        user: ink::primitives::AccountId, 
        cash_owner: ink::primitives::AccountId
    ) -> Result<(ink::primitives::AccountId, u128, u128, u128, u128, u128), crate::errors::LAssetError>;
}

#[ink::contract]
mod finance2 {
    use ink::prelude::vec::Vec;
    use ink::prelude::string::String;
    use traits::errors::FlashLoanPoolError;
    use traits::psp22::{PSP22, PSP22Error, PSP22Metadata, Transfer, Approval};
    use traits::FlashLoanPool;
    use crate::logic::{require, add, mulw, sub, Accruer};
    use crate::errors::LAssetError;

    use ink::storage::Mapping;
    use crate::LAsset;

    #[ink(storage)]
    pub struct LAssetContract {
        pub admin: AccountId,
        pub underlying_token: AccountId,
        pub last_updated_at: Timestamp,

        pub next: AccountId,

        pub total_collateral: u128,
        pub collateral: Mapping<AccountId, u128>,
    
        pub last_total_liquidity: u128,
        pub total_borrowable: u128,
    
        pub total_shares: u128,
        pub shares: Mapping<AccountId, u128>,
        pub allowance: Mapping<(AccountId, AccountId), u128>,
    
        pub total_bonds: u128,
        pub bonds: Mapping<AccountId, u128>,

        pub standard_rate: u128,
        pub standard_min_rate: u128,

        pub emergency_rate: u128,
        pub emergency_max_rate: u128,

        pub initial_margin: u128,
        pub maintenance_margin: u128,

        pub initial_haircut: u128,
        pub maintenance_haircut: u128,

        pub mint_fee: u128,
        pub borrow_fee: u128,
        pub liquidation_reward: u128,

        pub price: u128,
        pub price_scaler: u128,

        pub cash: Mapping<AccountId, u128>,
        pub whitelist: Mapping<AccountId, AccountId>,

        // PSP22Metadata
        pub name: Option<String>,
        pub symbol: Option<String>,
        pub decimals: u8,

        pub gas_collateral: u128,
    }

    impl LAssetContract {
        #[allow(clippy::too_many_arguments)]
        #[ink(constructor)]
        pub fn new(
            underlying_token: AccountId,
            next: AccountId,
            gas_collateral: u128,
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
                standard_rate: 0,
                standard_min_rate: 0,
                emergency_rate: 0,
                emergency_max_rate: 0,
                initial_margin: 0,
                maintenance_margin: 0,
                initial_haircut: u128::MAX,
                maintenance_haircut: u128::MAX,
                mint_fee: 0,
                borrow_fee: 0,
                liquidation_reward: 0,
                price: 1,
                price_scaler: 1,
                cash: Mapping::new(),
                whitelist: Mapping::new(),
                name,
                symbol,
                decimals,
                gas_collateral,
             }
        }
        #[allow(clippy::too_many_arguments)]
        pub fn set_params(
            &mut self,
            standard_rate: u128,
            standard_min_rate: u128,
            emergency_rate: u128,
            emergency_max_rate: u128,
            initial_margin: u128,
            maintenance_margin: u128,
            initial_haircut: u128,
            maintenance_haircut: u128,
            mint_fee: u128,
            borrow_fee: u128,
            liquidation_reward: u128,
        ) -> Result<(), LAssetError> {
            let caller = self.env().caller();
            require(caller == self.admin, LAssetError::SetParamsUnathorized)?;

            self.standard_rate = standard_rate;
            self.standard_min_rate = standard_min_rate;
            self.emergency_rate = emergency_rate;
            self.emergency_max_rate = emergency_max_rate;
            self.initial_margin = initial_margin;
            self.maintenance_margin = maintenance_margin;
            self.initial_haircut = initial_haircut;
            self.maintenance_haircut = maintenance_haircut;
            self.mint_fee = mint_fee;
            self.borrow_fee = borrow_fee;
            self.liquidation_reward = liquidation_reward;

            Ok(())
        }

        #[ink(message)]
        pub fn set_price(&mut self, price: u128, price_scaler: u128) -> Result<(), LAssetError> {
            let caller = self.env().caller();
            require(caller == self.admin, LAssetError::SetPriceUnathorized)?;
            
            self.price = price;
            self.price_scaler = price_scaler;

            Ok(())
        }

        #[ink(message)]
        pub fn deposit(&mut self, to_deposit: u128) -> Result<(), LAssetError> {
            let caller = self.env().caller();
            let this = self.env().account_id();
            transfer_from(self.underlying_token, caller, this, to_deposit).map_err(LAssetError::DepositTransferFailed)?;

            let collateral = if let Some(c) = self.collateral.get(caller) {
                Ok(c)
            } else if self.bonds.contains(caller) {
                Err(LAssetError::DepositWhileBorrowingNotAllowed)
            } else if self.env().transferred_value() != self.gas_collateral {
                Err(LAssetError::FirstDepositRequiresGasCollateral)
            } else {
                Ok(0)
            }?;
            let new_total_collateral = self.total_collateral.checked_add(to_deposit).ok_or(LAssetError::DepositOverflow)?;
            let new_collateral = add(collateral, to_deposit); //PROVED
            
            self.total_collateral = new_total_collateral;
            self.collateral.insert(caller, &new_collateral);
            
            Ok(())
        }
        
        #[ink(message)]
        pub fn withdraw(&mut self, to_withdraw: u128) -> Result<(), LAssetError> {
            let caller = self.env().caller();
            
            let collateral = self.collateral.get(caller).ok_or(LAssetError::WithdrawWithoutDeposit)?;
            let new_collateral = collateral.checked_sub(to_withdraw).ok_or(LAssetError::WithdrawOverflow)?;
            let new_total_collateral = sub(self.total_collateral, to_withdraw); //PROVED

            let mut total_icv = if let Some(qouted_collateral) = mulw(new_collateral, self.price).div(self.price_scaler) {
                mulw(qouted_collateral, self.initial_haircut).scale()
            } else {
                u128::MAX
            };
            let mut total_idv: u128 = 0;

            let mut current = self.next;
            let this = self.env().account_id();
            while current != this {
                let (next, icv, idv) = update_next(&current, &caller);
                current = next;
                total_icv = total_icv.saturating_add(icv);
                total_idv = total_idv.saturating_add(idv);
            }
            require(total_idv == 0 || total_icv > total_idv, LAssetError::CollateralValueTooLowAfterWithdraw)?;

            self.total_collateral = new_total_collateral;
            if new_collateral != 0 {
                self.collateral.insert(caller, &new_collateral);
            } else {
                self.collateral.remove(caller);
                self.env().transfer(caller, self.gas_collateral).ok().ok_or(LAssetError::WithdrawGasTransferFailed)?;
            }

            transfer(self.underlying_token, caller, to_withdraw).map_err(LAssetError::WithdrawTransferFailed)
        }

        #[ink(message)]
        pub fn mint(&mut self, to_wrap: u128) -> Result<(), LAssetError> {
            let caller = self.env().caller();
            let this = self.env().account_id();
            let extra_liquidity = mulw(to_wrap, self.mint_fee).scale_up().checked_add(to_wrap).ok_or(LAssetError::MintFeeOverflow)?;
            
            transfer_from(self.underlying_token, caller, this, extra_liquidity).map_err(LAssetError::MintTransferFailed)?;

            let total_borrowable = self.total_borrowable;
            let accruer = Accruer {
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
            
            let new_total_liquidity = total_liquidity.checked_add(extra_liquidity).ok_or(LAssetError::MintOverflow)?;
            let new_total_borrowable = add(total_borrowable, extra_liquidity); //PROVED
            
            let to_mint = mulw(to_wrap, total_shares).div_rate(total_liquidity).unwrap_or(extra_liquidity); //PROVED
            let new_total_shares = add(total_shares, to_mint); //PROVED
            let new_shares = add(shares, to_mint); //PROVED

            self.total_shares = new_total_shares;
            self.shares.insert(caller, &new_shares);
            
            self.total_borrowable = new_total_borrowable;
            self.last_total_liquidity = new_total_liquidity;
            self.last_updated_at = updated_at;
            
            self.env().emit_event(Transfer {from: None, to: Some(caller), value: to_mint});
            Ok(())
        }

        #[ink(message)]
        pub fn burn(&mut self, to_burn: u128) -> Result<(), LAssetError> {
            let caller = self.env().caller();

            let total_borrowable = self.total_borrowable;
            let accruer = Accruer {
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

            let new_shares = shares.checked_sub(to_burn).ok_or(LAssetError::BurnOverflow)?;
            let to_withdraw = mulw(to_burn, total_liquidity).div_rate(total_shares).unwrap_or(0); //PROVED
            let new_total_borrowable = total_borrowable.checked_sub(to_withdraw).ok_or(LAssetError::BurnTooMuch)?;
            let new_total_shares = sub(total_shares, to_burn); //PROVED
            let new_total_liquidity = sub(total_liquidity, to_withdraw); //PROVED

            self.total_shares = new_total_shares;
            self.shares.insert(caller, &new_shares);
            
            self.total_borrowable = new_total_borrowable;
            self.last_total_liquidity = new_total_liquidity;
            self.last_updated_at = updated_at;

            self.env().emit_event(Transfer {from: Some(caller), to: None, value: to_burn});

            transfer(self.underlying_token, caller, to_withdraw).map_err(LAssetError::BurnTransferFailed)
        }

        #[ink(message)]
        pub fn borrow(&mut self, to_borrow: u128) -> Result<(), LAssetError> {
            let caller = self.env().caller();
            let this = self.env().account_id();

            let total_borrowable = self.total_borrowable;
            let accruer = Accruer {
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

            let bonds = if let Some(b) = self.bonds.get(caller) {
                Ok(b)
            } else if self.collateral.contains(caller) {
                Err(LAssetError::BorrowWhileDepositingNotAllowed)
            } else if self.env().transferred_value() != self.gas_collateral {
                Err(LAssetError::FirstBorrowRequiresGasCollateral)
            } else {
                Ok(0)
            }?;

            let to_repay = mulw(to_borrow, self.borrow_fee).scale_up().checked_add(to_borrow).ok_or(LAssetError::BorrowFeeOverflow)?;
            let new_total_borrowable = total_borrowable.checked_sub(to_repay).ok_or(LAssetError::BorrowOverflow)?;
            let total_debt = sub(total_liquidity, total_borrowable); //PROVED
            let total_bonds = self.total_bonds;
            let to_mint = mulw(to_repay, total_bonds).ceil_rate(total_debt).unwrap_or(to_repay); //PROVED
            let new_total_bonds = add(total_bonds, to_mint); //PROVED
            let new_bonds = add(bonds, to_mint); //PROVED
            
            let new_total_debt = sub(total_liquidity, new_total_borrowable); //PROVED
            let debt = mulw(new_bonds, new_total_debt).ceil_rate(new_total_bonds).unwrap_or(new_total_debt); //PROVED
            let quoted_debt = mulw(debt, self.price).ceil_up(self.price_scaler).unwrap_or(u128::MAX);
            let mut total_idv = mulw(quoted_debt, self.initial_margin).scale_up().saturating_add(quoted_debt);
            let mut total_icv: u128 = 0;

            let mut current = self.next;
            while current != this {
                let (next, icv, idv) = update_next(&current, &caller);
                current = next;
                total_icv = total_icv.saturating_add(icv);
                total_idv = total_idv.saturating_add(idv);
            }
            require(total_icv > total_idv, LAssetError::CollateralValueTooLowAfterBorrow)?;

            self.total_bonds = new_total_bonds;
            self.bonds.insert(caller, &new_bonds);
            
            self.total_borrowable = new_total_borrowable;
            self.last_total_liquidity = total_liquidity;
            self.last_updated_at = updated_at;

            transfer(self.underlying_token, caller, to_borrow).map_err(LAssetError::BorrowTransferFailed)
        }

        #[ink(message)]
        pub fn deposit_cash(&mut self, spender: AccountId, extra_cash: u128) -> Result<(), LAssetError> {
            let caller = self.env().caller();
            let this = self.env().account_id();
            transfer_from(self.underlying_token, caller, this, extra_cash).map_err(LAssetError::DepositCashTransferFailed)?;
            
            let cash = self.cash.get(caller).unwrap_or(0);
            let new_cash = cash.checked_add(extra_cash).ok_or(LAssetError::DepositCashOverflow)?;

            self.cash.insert(caller, &new_cash);
            self.whitelist.insert(caller, &spender);

            Ok(())
        }

        #[ink(message)]
        pub fn withdraw_cash(&mut self) -> Result<(), LAssetError> {
            let caller = self.env().caller();
            let cash = self.cash.get(caller).unwrap_or(0);
            
            self.cash.remove(caller);

            transfer(self.underlying_token, caller, cash).map_err(LAssetError::WithdrawCashTransferFailed)
        }

        #[ink(message)]
        pub fn liquidate(&mut self, user: AccountId) -> Result<(), LAssetError> {
            let caller = self.env().caller();
            let this = self.env().account_id();

            let mut total_icv: u128 = 0;
            let mut total_idv: u128 = 0;
            let mut total_mcv: u128 = 0;
            let mut total_mdv: u128 = 0;
            let mut total_repaid: u128 = 0;

            let mut current = self.next;
            while current != this {
                let (next, repaid, icv, idv, mcv, mdv) = repay_or_update(current, user, caller)?;
                
                current = next;
                total_repaid = repaid.checked_add(repaid).ok_or(LAssetError::LiquidateTotalRepaidOverflow)?;
                total_icv = total_icv.saturating_add(icv);
                total_idv = total_idv.saturating_add(idv);
                total_mcv = total_mcv.saturating_add(mcv);
                total_mdv = total_mdv.saturating_add(mdv);
            }

            let accruer = Accruer {
                now: self.env().block_timestamp(),
                updated_at: self.last_updated_at,
                total_liquidity: self.last_total_liquidity,
                total_borrowable: self.total_borrowable,
                standard_rate: self.standard_rate,
                emergency_rate: self.emergency_rate,
                standard_min_rate: self.standard_min_rate,
                emergency_max_rate: self.emergency_max_rate,
            };
            let (total_liquidity, updated_at) = accruer.accrue();
            let collateral = self.collateral.get(user).ok_or(LAssetError::LiquidateForNothing)?;

            let price = self.price;
            let price_scaler = self.price_scaler;
            let repaid_collateral = mulw(total_repaid, price_scaler).div(price).ok_or(LAssetError::LiquidateRepaidCollateralOverflow)?;
            let rewards = mulw(repaid_collateral, self.liquidation_reward).scale_up();
            let to_take = repaid_collateral.checked_add(rewards).ok_or(LAssetError::LiquidateRewardsOverflow)?;

            let new_collateral = collateral.checked_sub(to_take).ok_or(LAssetError::LiquidateCollateralOverflow)?;
            let new_total_collateral = sub(self.total_collateral, to_take); //PROVED

            total_icv = if let Some(qouted_collateral) = mulw(collateral, price).div(price_scaler) {
                mulw(qouted_collateral, self.initial_haircut).scale().saturating_add(total_icv)
            } else {
                u128::MAX
            };
            total_mcv = if let Some(qouted_new_collateral) = mulw(new_collateral, price).div(price_scaler) {
                mulw(qouted_new_collateral, self.maintenance_haircut).scale().saturating_add(total_mcv)
            } else {
                u128::MAX
            };

            require(total_mdv < total_mcv, LAssetError::LiquidateTooEarly)?;
            require(total_idv < total_icv, LAssetError::LiquidateTooMuch)?;

            self.last_total_liquidity = total_liquidity;
            self.last_updated_at = updated_at;

            self.total_collateral = new_total_collateral;
            if new_collateral != 0 {
                self.collateral.insert(user, &new_collateral);
            } else {
                self.collateral.remove(user);
                self.env().transfer(caller, self.gas_collateral).ok().ok_or(LAssetError::LiquidateGasTransferFailed)?;
            }

            transfer(self.underlying_token, caller, to_take).map_err(LAssetError::LiquidateTransferFailed)
        }

        fn inner_repay(&mut self, caller: AccountId, user: AccountId, cash: u128
        ) -> Result<(u128, u128, u128, u128, u128), LAssetError> {
            let bonds = self.bonds.get(user).ok_or(LAssetError::RepayWithoutBorrow)?;
            let total_borrowable = self.total_borrowable;

            let accruer = Accruer {
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
            
            let total_debt = sub(total_liquidity, total_borrowable); //PROVED
            let total_bonds = self.total_bonds;
            let to_repay = mulw(cash, total_bonds).div_rate(total_debt).unwrap_or(0); //PROVED           
            let to_compensate = to_repay.saturating_sub(bonds);
            let to_burn = sub(to_repay, to_compensate); //PROVED

            let new_cash = mulw(to_compensate, total_debt).div_rate(total_bonds).unwrap_or(cash); //PROVED
            let new_bonds = sub(bonds, to_burn); //PROVED
            let repaid = sub(cash, new_cash); //PROVED

            let new_total_borrowable = add(total_borrowable, repaid); //PROVED
            let new_total_bonds = sub(total_bonds, to_burn); //PROVED

            self.cash.insert(caller, &new_cash);
            
            self.total_borrowable = new_total_borrowable;
            self.last_total_liquidity = total_liquidity;
            self.last_updated_at = updated_at;
            
            self.total_bonds = new_total_bonds;
            if new_bonds != 0 {
                self.bonds.insert(user, &new_bonds);
            } else {
                self.bonds.remove(user);
                self.env().transfer(caller, self.gas_collateral).ok().ok_or(LAssetError::RepayGasTransferFailed)?; //TODO: map_err
            }
            Ok((repaid, new_total_borrowable, new_total_bonds, new_bonds, total_liquidity))
        }


        #[ink(message)]
        pub fn repay(&mut self, user: AccountId, extra_cash: u128) -> Result<(), LAssetError> {
            let caller = self.env().caller();
            
            let this = self.env().account_id();
            transfer_from(self.underlying_token, caller, this, extra_cash).map_err(LAssetError::RepayTransferFailed)?;

            let cash = self.cash.get(caller).unwrap_or(0);
            let new_cash = cash.checked_add(extra_cash).ok_or(LAssetError::RepayCashOverflow)?;
            self.inner_repay(caller, user, new_cash)?;

            Ok(())
        }
    }

    impl LAsset for LAssetContract {
        #[ink(message)]
        fn repay_or_update(&mut self, user: AccountId, cash_owner: AccountId) -> Result<(AccountId, u128, u128, u128, u128, u128), LAssetError> {
            let caller = self.env().caller();

            let is_repay = if let Some(valid_caller) = self.whitelist.get(cash_owner) {
                valid_caller == caller
            } else {
                false
            };
            if is_repay {
                let price = self.price;
                let price_scaler = self.price_scaler;

                let cash = self.cash.get(cash_owner).unwrap_or(0);

                self.whitelist.remove(caller);

                let (repaid, new_borrowable, new_total_bonds, new_bonds, total_liquidity) = self.inner_repay(cash_owner, user, cash)?;
                let qouted_repaid = mulw(repaid, price).ceil_up(price_scaler).unwrap_or(u128::MAX);
                
                let total_debt = sub(total_liquidity, new_borrowable); //PROVED
                let debt = mulw(new_bonds, total_debt).ceil_up(new_total_bonds).unwrap_or(total_debt);
                let qouted_debt = mulw(debt, price).ceil_up(price_scaler).unwrap_or(u128::MAX);
                let mdv = mulw(qouted_debt, self.maintenance_margin).scale_up().saturating_add(qouted_debt);

                let old_debt = add(debt, repaid); //PROVED
                let old_qouted_debt = mulw(old_debt, price).ceil_up(price_scaler).unwrap_or(u128::MAX);
                let idv = mulw(old_qouted_debt, self.initial_margin).scale_up().saturating_add(old_qouted_debt);

                Ok((self.next, qouted_repaid, 0, idv, 0, mdv))
            } else {
                let total_borrowable = self.total_borrowable;
                let accurer = Accruer {
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

                self.last_total_liquidity = total_liquidity;
                self.last_updated_at = updated_at;
                
                if let Some(c) = self.collateral.get(user) {
                    if let Some(qouted_collateral) = mulw(c, self.price).div(self.price_scaler) {
                        let icv = mulw(qouted_collateral, self.initial_haircut).scale();
                        let mcv = mulw(qouted_collateral, self.maintenance_haircut).scale();
                        Ok((self.next, 0, icv, 0, mcv, 0))
                    } else {
                        Ok((self.next, 0, u128::MAX, 0, u128::MAX, 0))
                    }
                } else if let Some(b) = self.bonds.get(user) {
                    let total_debt = sub(total_liquidity, total_borrowable); //PROVED
                    let debt = mulw(b, total_debt).ceil_rate(self.total_bonds).unwrap_or(total_debt); //PROVED
                    let qouted_debt = mulw(debt, self.price).ceil_up(self.price_scaler).unwrap_or(u128::MAX);
                    let idv = mulw(qouted_debt, self.initial_margin).scale_up().saturating_add(qouted_debt);
                    let mdv = mulw(qouted_debt, self.maintenance_margin).scale_up().saturating_add(qouted_debt);
                    Ok((self.next, 0, 0, idv, 0, mdv))
                } else {
                    Ok((self.next, 0, 0, 0, 0, 0))
                }
            }
        }

        #[ink(message)]
        fn update(&mut self, user: AccountId) -> (AccountId, u128, u128) {
            let total_borrowable = self.total_borrowable;
            let accruer = Accruer {
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

            if let Some(c) = self.collateral.get(user) {
                if let Some(qouted_collateral) = mulw(c, self.price).div(self.price_scaler) {
                    let icv = mulw(qouted_collateral, self.initial_haircut).scale();
                    (self.next, icv, 0)
                } else {
                    (self.next, u128::MAX, 0)
                }
            } else if let Some(b) = self.bonds.get(user) {
                let total_debt = sub(total_liquidity, total_borrowable); //PROVED
                let debt = mulw(b, total_debt).ceil_rate(self.total_bonds).unwrap_or(total_debt); //PROVED
                let qouted_debt = mulw(debt, self.price).ceil_up(self.price_scaler).unwrap_or(u128::MAX);
                let idv = mulw(qouted_debt, self.initial_margin).scale_up().saturating_add(qouted_debt);
                (self.next, 0, idv)
            } else {
                (self.next, 0, 0)
            }
        }
    }

    impl FlashLoanPool for LAssetContract {
        #[ink(message)]
        fn take_cash(&mut self, amount: u128, target: AccountId) -> Result<AccountId, FlashLoanPoolError> {
            let caller = self.env().caller();
            require(caller == self.admin, FlashLoanPoolError::TakeCashUnauthorized)?;
            
            let underlying_token = self.underlying_token;
            transfer(underlying_token, target, amount).map_err(FlashLoanPoolError::TakeCashFailed)?;

            Ok(underlying_token)
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
            let new_from_shares = from_shares.checked_sub(value).ok_or(PSP22Error::InsufficientBalance)?;

            if from != to {
                let to_shares = self.shares.get(to).unwrap_or(0);
                let new_to_shares = add(to_shares, value);
                
                self.shares.insert(from, &new_from_shares);
                self.shares.insert(to, &new_to_shares);
                
                self.env().emit_event(Transfer {from: Some(from), to: Some(to), value});
            }
            Ok(())
        }

        #[ink(message)]
        fn transfer_from(&mut self, from: AccountId, to: AccountId, value: u128, _data: Vec<u8>) -> Result<(), PSP22Error> {
            let from_shares = self.shares.get(from).unwrap_or(0);
            let allowance = self.allowance.get((from, to)).unwrap_or(0);
            let new_allowance = allowance.checked_sub(value).ok_or(PSP22Error::InsufficientAllowance)?;
            let new_from_shares = from_shares.checked_sub(value).ok_or(PSP22Error::InsufficientBalance)?;
            
            if from != to {
                let to_shares = self.shares.get(to).unwrap_or(0);    
                let new_to_shares = add(to_shares, value); //PROVED

                self.shares.insert(from, &new_from_shares);
                self.shares.insert(to, &new_to_shares);
                self.allowance.insert((from, to), &new_allowance);

                self.env().emit_event(Approval {owner: from, spender: to, amount: new_allowance});
                self.env().emit_event(Transfer {from: Some(from), to: Some(to), value});
            }
            Ok(())
        }

        #[ink(message)]
        fn approve(&mut self, spender: AccountId, value: u128) -> Result<(), PSP22Error> {
            let owner = self.env().caller();
            self.allowance.insert((owner, spender), &value);
            
            self.env().emit_event(Approval { owner, spender, amount: value});
            Ok(())
        }

        #[ink(message)]
        fn increase_allowance(&mut self, spender: AccountId, delta_value: u128) -> Result<(), PSP22Error> {
            let owner = self.env().caller();
            let allowance = self.allowance.get((owner, spender)).unwrap_or(0);
            
            let new_allowance = allowance.saturating_add(delta_value);
            
            self.allowance.insert((owner, spender), &new_allowance);

            self.env().emit_event(Approval {owner, spender, amount: new_allowance});
            Ok(())
        }

        #[ink(message)]
        fn decrease_allowance(&mut self, spender: AccountId, delta_value: u128) -> Result<(), PSP22Error> {
            let owner = self.env().caller();
            let allowance = self.allowance.get((owner, spender)).unwrap_or(0);
            
            let new_allowance = allowance.saturating_sub(delta_value);

            self.allowance.insert((owner, spender), &new_allowance);

            self.env().emit_event(Approval {owner, spender, amount: new_allowance});
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
        const DEFAULT_DECIMALS: u8 = 6;
        use ink::codegen::TraitCallBuilder;
        let token: ink::contract_ref!(PSP22Metadata) = token.into();
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
    pub static mut L_BTC: Option<LAssetContract> = None;
    #[cfg(test)]
    pub static mut L_USDC: Option<LAssetContract> = None;
    #[cfg(test)]
    pub static mut L_ETH: Option<LAssetContract> = None;
    #[cfg(test)]
    pub static mut BALANCES: Option<std::collections::HashMap<(AccountId, AccountId), u128>> = None;
    #[cfg(test)]
    pub static mut CALLER: Option<AccountId> = None;
    #[cfg(test)]
    pub static mut CALLEE: Option<AccountId> = None;
    #[cfg(test)]
    pub static mut TRANSFER_ERROR: bool = false;

    #[cfg(not(test))]
    fn update_next(next: &AccountId, user: &AccountId) -> (AccountId, u128, u128) {
        let mut next: ink::contract_ref!(LAsset) = (*next).into();
        next.update(*user)
    }

    #[cfg(test)]
    fn get_next(next: &AccountId) -> &mut LAssetContract {
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(unsafe { CALLEE.unwrap() });
        ink::env::test::set_callee::<ink::env::DefaultEnvironment>(*next);
        unsafe {
            if *next == AccountId::from([0x3; 32]) {
                L_ETH.as_mut().unwrap()
            }
            else if *next == AccountId::from([0x2; 32]) {
                L_USDC.as_mut().unwrap()
            }
            else {
                L_BTC.as_mut().unwrap()
            }
        }
    }

    #[cfg(test)]
    fn restore_context() {
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(unsafe { CALLER.unwrap() });
        ink::env::test::set_callee::<ink::env::DefaultEnvironment>(unsafe { CALLEE.unwrap() });
    }

    #[cfg(test)]
    fn update_next(next: &AccountId, user: &AccountId) -> (AccountId, u128, u128) {
        let result = get_next(next).update(*user);
        restore_context();
        result
    }

    #[cfg(not(test))]
    fn repay_or_update(app: AccountId, user: AccountId, cash_owner: AccountId) -> Result<(AccountId, u128, u128, u128, u128, u128), LAssetError> {
        let mut app: ink::contract_ref!(LAsset) = app.into();
        app.repay_or_update(user, cash_owner)
    }
    #[cfg(test)]
    fn repay_or_update(app: AccountId, user: AccountId, cash_owner: AccountId) -> Result<(AccountId, u128, u128, u128, u128, u128), LAssetError> {
        let result = get_next(&app).repay_or_update(user, cash_owner);
        restore_context();
        result
    }

    #[cfg(not(test))]
    fn transfer_from(token: AccountId, from: AccountId, to: AccountId, value: u128) -> Result<(), PSP22Error> {
        let mut token: ink::contract_ref!(PSP22) = token.into();
        token.transfer_from(from, to, value, Vec::default())
    }
    #[cfg(test)]
    #[allow(unused_variables)]
    fn transfer_from(token: AccountId, from: AccountId, to: AccountId, value: u128) -> Result<(), PSP22Error> {
        let balances = unsafe { BALANCES.as_mut().unwrap() };
        let balance = balances.get(&(token, from)).unwrap_or(&0);
        let from_balance = balances.get(&(token, from)).unwrap_or(&0).checked_sub(value).ok_or(PSP22Error::InsufficientBalance)?;
        if from != to {
            let to_balance = balances.get(&(token, from)).unwrap_or(&0).saturating_add(value);
            balances.insert((token, from), from_balance);
            balances.insert((token, to), to_balance);
        }
        Ok(())
    }

    #[cfg(not(test))]
    fn transfer(token: AccountId, to: AccountId, value: u128) -> Result<(), PSP22Error> {
        let mut token: ink::contract_ref!(PSP22) = token.into();
        token.transfer(to, value, Vec::default())
    }
    #[cfg(test)]
    #[allow(unused_variables)]
    fn transfer(token: AccountId, to: AccountId, value: u128) -> Result<(), PSP22Error> {
        unsafe {
            if TRANSFER_ERROR {
                TRANSFER_ERROR = false;
                return Err(PSP22Error::Custom("".to_string()));
            }
        }

        let balances = unsafe { BALANCES.as_mut().unwrap() };
        let balance = balances.get(&(token, to)).unwrap_or(&0);
        let to_balance = balance.saturating_add(value);
        balances.insert((token, to), to_balance);
        Ok(())
    }
}

#[cfg(test)]
mod tests;