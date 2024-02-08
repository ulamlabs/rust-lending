#![cfg_attr(not(feature = "std"), no_std, no_main)]

pub mod errors;

use ink::primitives::AccountId;
use errors::FinanceError;

#[ink::trait_definition]
pub trait FinanceTrait {
    #[ink(message)]
    fn deposit(&mut self, token: AccountId, amount: u128) -> Result<(), FinanceError>;
    #[ink(message)]
    fn withdraw(&mut self, token: AccountId, amount: u128) -> Result<(), FinanceError>;
    #[ink(message)]
    fn invest(&mut self, token: AccountId, amount: u128) -> Result<(), FinanceError>;
    #[ink(message)]
    fn redeposit(&mut self, token: AccountId, amount: u128) -> Result<(), FinanceError>;
    #[ink(message)]
    fn borrow(&mut self, token: AccountId, amount: u128) -> Result<(), FinanceError>;
    #[ink(message)]
    fn redeem(&mut self, token: AccountId, amount: u128) -> Result<(), FinanceError>;
    #[ink(message)]
    fn update_price(&mut self, token: AccountId, user: AccountId, price: u128) -> Result<(), FinanceError>;
    #[ink(message)]
    fn disable(&mut self, token: AccountId) -> Result<(), FinanceError>;
    #[ink(message)]
    fn enable(&mut self, token: AccountId) -> Result<(), FinanceError>;
}