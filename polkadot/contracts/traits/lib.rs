#![cfg_attr(not(feature = "std"), no_std, no_main)]

pub mod errors;

use ink::primitives::AccountId;
use ink::prelude::vec::Vec;
pub use errors::FinanceError;

#[derive(scale::Encode, scale::Decode, scale_info::TypeInfo)]
pub enum FinanceAction {
    Deposit,
    Withdraw,
    Invest,
    Redeposit,
    Borrow,
    Repay
}

#[ink::trait_definition]
pub trait FinanceTrait {
    #[ink(message)]
    fn disable(&mut self, token: AccountId) -> Result<(), FinanceError>;
    #[ink(message)]
    fn enable(&mut self, token: AccountId, address: AccountId) -> Result<(), FinanceError>;
    #[ink(message)]
    fn update(&mut self, action: FinanceAction, user:AccountId, token: AccountId, amount: u128, tokens: Vec<AccountId>) -> Result<(), FinanceError>;
    #[ink(message)]
    fn set_price(&mut self, token: AccountId, price: u128) -> Result<(), FinanceError>;
    /// Distribute yield to the asset pool contract, this only needs to happen if invested amount changes
    /// Returns:
    /// - Yield distributed in base units (as new debt)
    #[ink(message)]
    fn distribute_yield(&mut self, token: AccountId, delta_invested: i128) -> Result<u128, FinanceError>;
}