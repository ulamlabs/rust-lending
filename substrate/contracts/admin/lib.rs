#![cfg_attr(not(feature = "std"), no_std, no_main)]

use errors::FlashLoanReceiverError;
use ink::primitives::AccountId;
use ink::prelude::vec::Vec;

mod errors;

#[ink::trait_definition]
pub trait FlashLoanReceiver {
    /// Interface for the flash loan receiver contract
    /// The recipient must increase allowance in the calling contract by amount + fee
    /// This interface is based on EIP-3156 (https://eips.ethereum.org/EIPS/eip-3156)
    #[ink(message)]
    fn on_flash_loan(&mut self, initiator: AccountId, token: AccountId, amount: u128, fee: u128, data: Vec<u8>) -> Result<(), FlashLoanReceiverError>;
}

#[ink::contract]
mod admin {
    use finance2::logic::require;
    use finance2::LAssetContractRef;
    use finance2::structs::{AssetParams, AssetPool, LAsset, UpdateOrRepayResult, UpdateResult};
    use ink::contract_ref;
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;
    use crate::FlashLoanReceiver;
    use traits::psp22::PSP22;
    use crate::errors::{AdminError, FlashLoanError};

    #[ink(storage)]
    pub struct Admin {
        pub dao: AccountId, // TODO: to be removed, when real dao is implemented
        pub hash: Hash,
        pub next: AccountId,

        pub prices: Mapping<AccountId, (u128, u128)>,
        pub params: Mapping<AccountId, AssetParams>,
    }

    impl Admin {
        #[ink(constructor)]
        pub fn new(hash: Hash) -> Self {
            let dao = Self::env().caller();
            let next = Self::env().account_id();
            Self {
                dao,
                hash,
                next,
                prices: Mapping::new(),
                params: Mapping::new(),
            }
        }

        #[ink(message)]
        pub fn add_asset(&mut self, underlying: AccountId, gas_collateral: u128) -> Result<(), AdminError> {
            let caller = self.env().caller();
            require(caller == self.dao, AdminError::AddAssetUnauthorized)?;

            let salt: [u8; 32] = *underlying.as_ref();
            let builder = LAssetContractRef::new(underlying, self.next, gas_collateral);
            let instantiator = builder.salt_bytes(salt).code_hash(self.hash).endowment(0);
            let contract = instantiator.instantiate();
            
            self.next = *contract.as_ref();
            Ok(())
        }

        #[ink(message)]
        pub fn pull_prices(&self) {
            let this = self.env().account_id();
            let mut current = self.next;
            while current != this {
                let mut asset: contract_ref!(AssetPool) = current.into();
                let (price, price_scaler) = self.prices.get(current).unwrap_or((0, 1)); // TODO: use chainlink or switchboard instead
                current = asset.set_price(price, price_scaler).unwrap(); //impossible to fail
            }
        }

        #[ink(message)]
        pub fn push_price(&mut self, asset: AccountId, price: u128, price_scaler: u128) -> Result<(), AdminError> {
            let caller = self.env().caller();
            require(caller == self.dao, AdminError::PushPriceUnauthorized)?; // TODO: use pyth payload instead

            self.prices.insert(asset, &(price, price_scaler));
            Ok(())
        }

        #[ink(message)]
        pub fn push_params(&mut self, asset: AccountId, params: AssetParams) -> Result<(), AdminError> {
            let caller = self.env().caller();
            require(caller == self.dao, AdminError::PushParamsUnauthorized)?;

            self.params.insert(asset, &params);
            Ok(())
        }

        #[ink(message)]
        pub fn pull_params(&self) {
            let this = self.env().account_id();
            let mut current = self.next;
            while current != this {
                let mut asset: contract_ref!(AssetPool) = current.into();
                let params = self.params.get(current).unwrap_or_default();
                current = asset.set_params(params).unwrap(); //impossible to fail
            }
        }

        #[ink(message)]
        pub fn flash_loan(&mut self, target_address: AccountId, pool_address: AccountId, amount: u128, data: Vec<u8>) -> Result<(), FlashLoanError>{
            let mut pool: contract_ref!(AssetPool) = pool_address.into();
            let caller = self.env().caller();

            let (underlying, fee) = pool.take_cash(amount, target_address).map_err(FlashLoanError::TakeCashFailed)?;
            let mut underlying_ref: contract_ref!(PSP22) = underlying.into();

            let mut target: contract_ref!(FlashLoanReceiver) = target_address.into();
            target.on_flash_loan(caller, underlying, amount, fee, data).map_err(FlashLoanError::ReceiverFailed)?;

            let new_amount = amount.checked_add(fee).ok_or(FlashLoanError::Overflow)?;
            underlying_ref.transfer_from(target_address, pool_address, new_amount, Vec::new()).map_err(FlashLoanError::TransferFailed)?;
            
            Ok(())
        }
    }

    impl LAsset for Admin {
        #[ink(message)]
        fn update(&mut self, _user: AccountId) -> UpdateResult {
            UpdateResult::new(self.next) // it is possible to block withdraw and borrow from here
        }

        #[ink(message)]
        fn repay_or_update(&mut self, _user: AccountId, _cash_owner: AccountId) -> UpdateOrRepayResult {
            UpdateOrRepayResult::new(self.next) // it is possible to block liquidate from here
        }
    }
}
