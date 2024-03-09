#![cfg_attr(not(feature = "std"), no_std, no_main)]

pub mod logic;
pub mod errors;
pub mod structs;

pub use self::finance2::LAssetContractRef;

#[ink::contract]
mod finance2 {
    use ink::prelude::vec::Vec;
    use ink::prelude::string::{String, ToString};
    use traits::psp22::{PSP22, PSP22Error, PSP22Metadata, Transfer, Approval};
    use crate::errors::TakeCashError;
    use crate::logic::{require, add, mulw, sub};
    use crate::errors::LAssetError;
    use crate::structs::{AssetParams, AssetPool, UpdateResult, LAsset};
    use ink::storage::Mapping;

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

        pub params: AssetParams,

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
                params: AssetParams {
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
                    take_cash_fee: 0,
                    liquidation_reward: 0,
                },
                price: 0,
                price_scaler: 1,
                cash: Mapping::new(),
                whitelist: Mapping::new(),
                name,
                symbol,
                decimals,
                gas_collateral,
             }
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
                mulw(qouted_collateral, self.params.initial_haircut).scale()
            } else {
                u128::MAX
            };
            let mut total_idv: u128 = 0;

            let mut next = self.next;
            let this = self.env().account_id();
            while next != this {
                let result = update_next(&next, &caller);
                next = result.next;
                total_icv = total_icv.saturating_add(result.initial_collateral_value);
                total_idv = total_idv.saturating_add(result.initial_debt_value);
            }
            require(total_idv == 0 || total_icv > total_idv, LAssetError::CollateralValueTooLowAfterWithdraw)?;

            self.total_collateral = new_total_collateral;
            if new_collateral != 0 {
                self.collateral.insert(caller, &new_collateral);
            } else {
                self.collateral.remove(caller);
                self.transfer_gas(caller);
            }

            transfer(self.underlying_token, caller, to_withdraw).map_err(LAssetError::WithdrawTransferFailed)
        }

        fn transfer_gas(&self, to: AccountId) {
            let _ = self.env().transfer(to, self.gas_collateral); //should never fail
        }

        #[ink(message)]
        pub fn mint(&mut self, to_wrap: u128) -> Result<(), LAssetError> {
            let caller = self.env().caller();
            let this = self.env().account_id();

            let fee = mulw(to_wrap, self.params.mint_fee).scale_up();
            let to_transfer = to_wrap.checked_add(fee).ok_or(LAssetError::MintFeeOverflow)?;
            
            transfer_from(self.underlying_token, caller, this, to_transfer).map_err(LAssetError::MintTransferFailed)?;

            let total_borrowable = self.total_borrowable;
            let (total_liquidity, updated_at) = self.inner_accrue(total_borrowable);

            let total_shares = self.total_shares;
            let shares = self.shares.get(caller).unwrap_or(0);
            
            let new_total_liquidity = total_liquidity.checked_add(to_transfer).ok_or(LAssetError::MintOverflow)?;
            let new_total_borrowable = add(total_borrowable, to_transfer); //PROVED
            
            let to_mint = mulw(to_wrap, total_shares).div_rate(total_liquidity).unwrap_or(to_transfer); //PROVED
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
            let (total_liquidity, updated_at) = self.inner_accrue(total_borrowable);

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
            let (total_liquidity, updated_at) = self.inner_accrue(total_borrowable);

            let bonds = if let Some(b) = self.bonds.get(caller) {
                Ok(b)
            } else if self.collateral.contains(caller) {
                Err(LAssetError::BorrowWhileDepositingNotAllowed)
            } else if self.env().transferred_value() != self.gas_collateral {
                Err(LAssetError::FirstBorrowRequiresGasCollateral)
            } else {
                Ok(0)
            }?;

            let fee = mulw(to_borrow, self.params.borrow_fee).scale_up();
            let to_return = to_borrow.checked_add(fee).ok_or(LAssetError::BorrowFeeOverflow)?;
            let new_total_borrowable = total_borrowable.checked_sub(to_return).ok_or(LAssetError::BorrowOverflow)?;
            let total_debt = sub(total_liquidity, total_borrowable); //PROVED
            let total_bonds = self.total_bonds;
            let to_mint = mulw(to_return, total_bonds).ceil_rate(total_debt).unwrap_or(to_return); //PROVED
            let new_total_bonds = add(total_bonds, to_mint); //PROVED
            let new_bonds = add(bonds, to_mint); //PROVED
            
            let new_total_debt = sub(total_liquidity, new_total_borrowable); //PROVED
            let debt = mulw(new_bonds, new_total_debt).ceil_rate(new_total_bonds).unwrap_or(new_total_debt); //PROVED
            let quoted_debt = mulw(debt, self.price).ceil_up(self.price_scaler).unwrap_or(u128::MAX);
            let mut total_idv = mulw(quoted_debt, self.params.initial_margin).scale_up().saturating_add(quoted_debt);
            let mut total_icv: u128 = 0;

            let mut next = self.next;
            while next != this {
                let result = update_next(&next, &caller);
                next = result.next;
                total_icv = total_icv.saturating_add(result.initial_collateral_value);
                total_idv = total_idv.saturating_add(result.initial_debt_value);
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
                let (next, repaid, icv, idv, mcv, mdv) = repay_or_update(current, user, caller);
                
                current = next;
                total_repaid = repaid.saturating_add(repaid);
                total_icv = total_icv.saturating_add(icv);
                total_idv = total_idv.saturating_add(idv);
                total_mcv = total_mcv.saturating_add(mcv);
                total_mdv = total_mdv.saturating_add(mdv);
            }

            let collateral = self.collateral.get(user).ok_or(LAssetError::LiquidateForNothing)?;

            let price = self.price;
            let price_scaler = self.price_scaler;
            let repaid_collateral = mulw(total_repaid, price_scaler).div(price).unwrap_or(u128::MAX);
            let rewards = mulw(repaid_collateral, self.params.liquidation_reward).scale_up();
            let to_take = repaid_collateral.saturating_add(rewards).min(collateral);

            let new_collateral = sub(collateral, to_take); //PROVED
            let new_total_collateral = sub(self.total_collateral, to_take); //PROVED

            total_icv = if let Some(qouted_collateral) = mulw(collateral, price).div(price_scaler) {
                mulw(qouted_collateral, self.params.initial_haircut).scale().saturating_add(total_icv)
            } else {
                u128::MAX
            };
            total_mcv = if let Some(qouted_new_collateral) = mulw(new_collateral, price).div(price_scaler) {
                mulw(qouted_new_collateral, self.params.maintenance_haircut).scale().saturating_add(total_mcv)
            } else {
                u128::MAX
            };

            require(total_mdv > total_mcv, LAssetError::LiquidateTooEarly)?;
            require(total_idv > total_icv, LAssetError::LiquidateTooMuch)?;

            self.total_collateral = new_total_collateral;
            if new_collateral != 0 {
                self.collateral.insert(user, &new_collateral);
            } else {
                self.collateral.remove(user);
                self.transfer_gas(caller);
            }

            transfer(self.underlying_token, caller, to_take).map_err(LAssetError::LiquidateTransferFailed)
        }

        fn inner_repay(&mut self, 
            caller: AccountId, 
            user: AccountId, 
            cash: u128,
            bonds: u128,
        ) -> (u128, u128, u128, u128, u128) {
            let total_borrowable = self.total_borrowable;
            let (total_liquidity, updated_at) = self.inner_accrue(total_borrowable);
            
            let total_borrowable = self.total_borrowable;
            let total_debt = sub(total_liquidity, total_borrowable); //PROVED
            let total_bonds = self.total_bonds;

            let max_to_burn = mulw(cash, total_bonds).div_rate(total_debt).unwrap_or(0); //PROVED
            let to_burn = max_to_burn.min(bonds); //PROVED
            let repaid = mulw(to_burn, total_debt).ceil_rate(total_bonds).unwrap_or(0); //PROVED
            let new_cash = sub(cash, repaid); //PROVED

            let new_total_borrowable = add(total_borrowable, repaid); //PROVED
            let new_bonds = sub(bonds, to_burn); //PROVED
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
                self.transfer_gas(caller);
            }
            (repaid, new_total_borrowable, new_total_bonds, new_bonds, total_liquidity)
        }

        #[ink(message)]
        pub fn accrue(&mut self) -> Result<(), LAssetError> {
            let (total_liquidity, updated_at) = self.inner_accrue(self.total_borrowable);

            self.last_total_liquidity = total_liquidity;
            self.last_updated_at = updated_at;

            Ok(())
        }

        #[ink(message)]
        pub fn repay(&mut self, user: AccountId, extra_cash: u128) -> Result<(), LAssetError> {
            let caller = self.env().caller();
            
            let this = self.env().account_id();
            transfer_from(self.underlying_token, caller, this, extra_cash).map_err(LAssetError::RepayTransferFailed)?;

            let cash = self.cash.get(caller).unwrap_or(0);
            let new_cash = cash.checked_add(extra_cash).ok_or(LAssetError::RepayCashOverflow)?;
            let bonds = self.bonds.get(user).ok_or(LAssetError::RepayWithoutBorrow)?;
            self.inner_repay(caller, user, new_cash, bonds);

            Ok(())
        }

        fn inner_accrue(&self, total_borrowable: u128) -> (u128, u64) {
            let now = self.env().block_timestamp();
            let updated_at = self.last_updated_at;
            let total_liquidity = self.last_total_liquidity;
            if now > updated_at {
                let delta = sub(now as u128, updated_at as u128);
                let standard_matured = self.params.standard_rate.saturating_mul(delta);
                let emergency_matured = self.params.emergency_rate.saturating_mul(delta);
    
                let debt = sub(total_liquidity, total_borrowable);
    
                let standard_scaled = mulw(standard_matured, debt).div_rate(total_liquidity).unwrap_or(0);
                let emergency_scaled = mulw(emergency_matured, total_borrowable).div_rate(total_liquidity).unwrap_or(0);
    
                let standard_final = standard_scaled.saturating_add(self.params.standard_min_rate);
                let emergency_final = self.params.emergency_max_rate.saturating_sub(emergency_scaled);
    
                let interest_rate = standard_final.max(emergency_final);
                let interest = mulw(debt, interest_rate).scale_up();
    
                let new_total_liquidity = total_liquidity.saturating_add(interest);
                (new_total_liquidity, now)    
            } else {
                (total_liquidity, updated_at)
            }
        }        
    }

    impl LAsset for LAssetContract {
        #[ink(message)]
        fn repay_or_update(&mut self, user: AccountId, cash_owner: AccountId) -> (AccountId, u128, u128, u128, u128, u128) {
            let caller = self.env().caller();

            let is_repay = if let Some(valid_caller) = self.whitelist.get(cash_owner) {
                if valid_caller == caller {
                    self.bonds.get(user)
                } else {
                    None
                }
            } else {
                None
            };
            if let Some(bonds) = is_repay {
                let price = self.price;
                let price_scaler = self.price_scaler;

                let cash = self.cash.get(cash_owner).unwrap_or(0);

                self.whitelist.remove(caller);

                let (repaid, new_borrowable, new_total_bonds, new_bonds, total_liquidity) = self.inner_repay(cash_owner, user, cash, bonds);
                let qouted_repaid = mulw(repaid, price).ceil_up(price_scaler).unwrap_or(u128::MAX);
                
                let total_debt = sub(total_liquidity, new_borrowable); //PROVED
                let debt = mulw(new_bonds, total_debt).ceil_up(new_total_bonds).unwrap_or(total_debt);
                let qouted_debt = mulw(debt, price).ceil_up(price_scaler).unwrap_or(u128::MAX);
                let mdv = mulw(qouted_debt, self.params.maintenance_margin).scale_up().saturating_add(qouted_debt);

                let old_debt = add(debt, repaid); //PROVED
                let old_qouted_debt = mulw(old_debt, price).ceil_up(price_scaler).unwrap_or(u128::MAX);
                let idv = mulw(old_qouted_debt, self.params.initial_margin).scale_up().saturating_add(old_qouted_debt);

                (self.next, qouted_repaid, 0, idv, 0, mdv)
            } else if let Some(c) = self.collateral.get(user) {
                if let Some(qouted_collateral) = mulw(c, self.price).div(self.price_scaler) {
                    let icv = mulw(qouted_collateral, self.params.initial_haircut).scale();
                    let mcv = mulw(qouted_collateral, self.params.maintenance_haircut).scale();
                    (self.next, 0, icv, 0, mcv, 0)
                } else {
                    (self.next, 0, u128::MAX, 0, u128::MAX, 0)
                }
            } else if let Some(b) = self.bonds.get(user) {
                let total_borrowable = self.total_borrowable;
                let (total_liquidity, updated_at) = self.inner_accrue(total_borrowable);

                self.last_total_liquidity = total_liquidity;
                self.last_updated_at = updated_at;

                let total_debt = sub(total_liquidity, total_borrowable); //PROVED
                let debt = mulw(b, total_debt).ceil_rate(self.total_bonds).unwrap_or(total_debt); //PROVED
                let qouted_debt = mulw(debt, self.price).ceil_up(self.price_scaler).unwrap_or(u128::MAX);
                let idv = mulw(qouted_debt, self.params.initial_margin).scale_up().saturating_add(qouted_debt);
                let mdv = mulw(qouted_debt, self.params.maintenance_margin).scale_up().saturating_add(qouted_debt);
                (self.next, 0, 0, idv, 0, mdv)
            } else {
                (self.next, 0, 0, 0, 0, 0)
            }
        }

        #[ink(message)]
        fn update(&mut self, user: AccountId) -> UpdateResult {
            if let Some(c) = self.collateral.get(user) {
                if let Some(qouted_collateral) = mulw(c, self.price).div(self.price_scaler) {
                    let icv = mulw(qouted_collateral, self.params.initial_haircut).scale();
                    UpdateResult::from_collateral(self.next, icv)
                } else {
                    UpdateResult::from_collateral(self.next, u128::MAX)
                }
            } else if let Some(b) = self.bonds.get(user) {
                let total_borrowable = self.total_borrowable;
                let (total_liquidity, updated_at) = self.inner_accrue(total_borrowable);

                self.last_total_liquidity = total_liquidity;
                self.last_updated_at = updated_at;

                let total_debt = sub(total_liquidity, total_borrowable); //PROVED
                let debt = mulw(b, total_debt).ceil_rate(self.total_bonds).unwrap_or(total_debt); //PROVED
                let qouted_debt = mulw(debt, self.price).ceil_up(self.price_scaler).unwrap_or(u128::MAX);
                let idv = mulw(qouted_debt, self.params.initial_margin).scale_up().saturating_add(qouted_debt);
                UpdateResult::from_debt(self.next, idv)
            } else {
                UpdateResult::new(self.next)
            }
        }
    }
    impl AssetPool for LAssetContract {
        #[ink(message)]
        fn take_cash(&mut self, amount: u128, target: AccountId) -> Result<(AccountId, u128), TakeCashError> {
            let caller = self.env().caller();
            require(caller == self.admin, TakeCashError::Unauthorized)?;
            
            let fee = mulw(amount, self.params.take_cash_fee).scale_up();
            let new_total_liquidity = self.last_total_liquidity.checked_add(fee).ok_or(TakeCashError::Overflow)?;
            let new_total_borrowable = add(self.total_borrowable, fee); //PROVED

            self.last_total_liquidity = new_total_liquidity;
            self.total_borrowable = new_total_borrowable;

            let underlying_token = self.underlying_token;
            transfer(underlying_token, target, amount).map_err(TakeCashError::Transfer)?;

            Ok((underlying_token, fee))
        }
        
        #[ink(message)]
        fn set_price(&mut self, price: u128, price_scaler: u128) -> Result<AccountId, LAssetError> {
            let caller = self.env().caller();
            require(caller == self.admin, LAssetError::SetPriceUnathorized)?;
            
            self.price = price;
            self.price_scaler = price_scaler;
            
            Ok(self.next)
        }
        
        #[ink(message)]
        fn set_params(&mut self, params: AssetParams) -> Result<AccountId, LAssetError> {
            let caller = self.env().caller();
            require(caller == self.admin, LAssetError::SetParamsUnathorized)?;

            self.params = params;
            Ok(self.next)
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

    #[cfg(not(any(test, fuzzing)))]
    fn fetch_psp22_metadata(token: AccountId) -> (Option<String>, Option<String>, u8) {
        const DEFAULT_DECIMALS: u8 = 6;
        use ink::codegen::TraitCallBuilder;
        use core::ops::Add;
        let token: ink::contract_ref!(PSP22Metadata) = token.into();
        let l_name = match token.call().token_name().try_invoke() {
            Ok(Ok(Some(name))) => Some("L-".to_string().add(name.as_str())),
            _ => None,
        };
        let l_symbol = match token.call().token_symbol().try_invoke() {
            Ok(Ok(Some(symbol))) => Some("L-".to_string().add(symbol.as_str())),
            _ => None,
        };
        let decimals = match token.call().token_decimals().try_invoke() {
            Ok(Ok(decimals)) => decimals,
            _ => DEFAULT_DECIMALS,
        };

        (l_name, l_symbol, decimals)
    }

    #[cfg(any(test, fuzzing))]
    #[allow(unused_variables)]
    fn fetch_psp22_metadata(token: AccountId) -> (Option<String>, Option<String>, u8) {
        (Some("L-TestToken".to_string()), Some("L-TT".to_string()), 16)
    }

    #[cfg(any(test, fuzzing))]
    pub static mut L_BTC: Option<LAssetContract> = None;
    #[cfg(any(test, fuzzing))]
    pub static mut L_USDC: Option<LAssetContract> = None;
    #[cfg(any(test, fuzzing))]
    pub static mut L_ETH: Option<LAssetContract> = None;
    #[cfg(any(test, fuzzing))]
    pub static mut BALANCES: Option<std::collections::HashMap<(AccountId, AccountId), u128>> = None;
    #[cfg(any(test, fuzzing))]
    pub static mut CALLER: Option<AccountId> = None;
    #[cfg(any(test, fuzzing))]
    pub static mut CALLEE: Option<AccountId> = None;
    #[cfg(any(test, fuzzing))]
    pub static mut TRANSFER_ERROR: bool = false;
    #[cfg(any(test, fuzzing))]
    pub const BTC_ADDRESS: [u8; 32] = [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31];
    #[cfg(any(test, fuzzing))]
    pub const ETH_ADDRESS: [u8; 32] = [1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,0];
    #[cfg(any(test, fuzzing))]
    pub const USDC_ADDRESS: [u8; 32] = [2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,0,1];

    #[cfg(not(any(test, fuzzing)))]
    fn update_next(next: &AccountId, user: &AccountId) -> UpdateResult {
        let mut next: ink::contract_ref!(LAsset) = (*next).into();
        next.update(*user)
    }

    #[cfg(any(test, fuzzing))]
    fn get_next(next: &AccountId) -> &mut LAssetContract {
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(unsafe { CALLEE.unwrap() });
        ink::env::test::set_callee::<ink::env::DefaultEnvironment>(*next);
        unsafe {
            if *next == AccountId::from(ETH_ADDRESS) {
                L_ETH.as_mut().unwrap()
            }
            else if *next == AccountId::from(BTC_ADDRESS) {
                L_BTC.as_mut().unwrap()
            }
            else {
                L_USDC.as_mut().unwrap()
            }
        }
    }

    #[cfg(any(test, fuzzing))]
    fn restore_context() {
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(unsafe { CALLER.unwrap() });
        ink::env::test::set_callee::<ink::env::DefaultEnvironment>(unsafe { CALLEE.unwrap() });
    }

    #[cfg(any(test, fuzzing))]
    fn update_next(next: &AccountId, user: &AccountId) -> UpdateResult {
        let result = get_next(next).update(*user);
        restore_context();
        result
    }

    #[cfg(not(any(test, fuzzing)))]
    fn repay_or_update(app: AccountId, user: AccountId, cash_owner: AccountId) -> (AccountId, u128, u128, u128, u128, u128) {
        let mut app: ink::contract_ref!(LAsset) = app.into();
        app.repay_or_update(user, cash_owner)
    }
    #[cfg(any(test, fuzzing))]
    fn repay_or_update(app: AccountId, user: AccountId, cash_owner: AccountId) -> (AccountId, u128, u128, u128, u128, u128) {
        let result = get_next(&app).repay_or_update(user, cash_owner);
        restore_context();
        result
    }

    #[cfg(not(any(test, fuzzing)))]
    fn transfer_from(token: AccountId, from: AccountId, to: AccountId, value: u128) -> Result<(), PSP22Error> {
        let mut token: ink::contract_ref!(PSP22) = token.into();
        token.transfer_from(from, to, value, Vec::default())
    }
    #[cfg(any(test, fuzzing))]
    fn transfer_from(token: AccountId, from: AccountId, to: AccountId, value: u128) -> Result<(), PSP22Error> {
        let balances = unsafe { BALANCES.as_mut().unwrap() };
        let from_balance = balances.get(&(token, from)).unwrap_or(&0).checked_sub(value).ok_or(PSP22Error::InsufficientBalance)?;
        if from != to {
            let to_balance = balances.get(&(token, from)).unwrap_or(&0).saturating_add(value);
            balances.insert((token, from), from_balance);
            balances.insert((token, to), to_balance);
        }
        Ok(())
    }

    #[cfg(not(any(test, fuzzing)))]
    fn transfer(token: AccountId, to: AccountId, value: u128) -> Result<(), PSP22Error> {
        let mut token: ink::contract_ref!(PSP22) = token.into();
        token.transfer(to, value, Vec::default())
    }
    
    #[cfg(any(test, fuzzing))]
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

#[cfg(any(test, fuzzing))]
pub mod tests;