use crate::errors::{LAssetError, TakeCashError};
use ink::primitives::AccountId;

#[derive(Debug)]
#[ink::scale_derive(Encode, Decode, TypeInfo)]
pub struct UpdateResult {
    pub next: AccountId,
    pub initial_collateral_value: u128,
    pub initial_debt_value: u128,
}

impl UpdateResult {
    pub fn from_collateral(next: AccountId, icv: u128) -> Self {
        Self { 
            next,
            initial_collateral_value: icv, 
            initial_debt_value: 0 
        }
    }
    pub fn from_debt(next: AccountId, idv: u128) -> Self {
        Self { 
            next,
            initial_collateral_value: 0, 
            initial_debt_value: idv 
        }
    }
    pub fn new(next: AccountId) -> Self {
        Self { 
            next,
            initial_collateral_value: 0, 
            initial_debt_value: 0 
        }
    }
}

#[ink::trait_definition]
pub trait LAsset {
    #[ink(message)]
    fn update(&mut self, user: AccountId) -> UpdateResult;

    #[ink(message)]
    fn repay_or_update(&mut self, user: AccountId, cash_owner: AccountId) -> (AccountId, u128, u128, u128, u128, u128);
}

#[ink::trait_definition]
pub trait AssetPool {
    #[ink(message)]
    fn take_cash(&mut self, amount: u128, target: AccountId) -> Result<(AccountId, u128), TakeCashError>;
    
    #[ink(message)]
    fn set_price(&mut self, price: u128, price_scaler: u128) -> Result<AccountId, LAssetError>;
    
    #[ink(message)]
    fn set_params(&mut self, params: AssetParams) -> Result<AccountId, LAssetError>;
}

#[derive(Debug, Default)]
#[ink::scale_derive(Encode, Decode, TypeInfo)]
#[cfg_attr(feature = "std", derive(ink::storage::traits::StorageLayout))]

pub struct AssetParams {
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
    pub take_cash_fee: u128,
    pub liquidation_reward: u128,
}
