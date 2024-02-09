#![cfg_attr(not(feature = "std"), no_std, no_main)]

pub mod errors;

use ink::primitives::AccountId;
use ink::prelude::vec::Vec;
use errors::FinanceError;

#[ink::trait_definition]
pub trait FinanceTrait {
    #[ink(message)]
    fn disable(&mut self, token: AccountId) -> Result<(), FinanceError>;
    #[ink(message)]
    fn enable(&mut self, token: AccountId, address: AccountId) -> Result<(), FinanceError>;
    #[ink(message)]
    fn update(&mut self, action: u8, user:AccountId, token: AccountId, amount: u128, tokens: Vec<AccountId>) -> Result<(), FinanceError>;
    #[ink(message)]
    fn set_price(&mut self, token: AccountId, price: u128) -> Result<(), FinanceError>;
}