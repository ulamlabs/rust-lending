#![cfg_attr(not(feature = "std"), no_std, no_main)]

pub mod errors;
pub mod psp22;

use ink::primitives::AccountId;
pub use errors::{LAssetError, FlashCalleeError};

#[ink::trait_definition]
pub trait FlashLoanPool {
    /// Authorized method: only the flash loan contract can call it to receive funds
    #[ink(message)]
    fn take_cash(&mut self, amount: u128) -> Result<(), LAssetError>;

    #[ink(message)]
    fn underlying_token(&self) -> AccountId;
}

#[ink::trait_definition]
pub trait FlashLoanContract { 
    #[ink(message)]
    fn flash_loan(&mut self, pool_address: AccountId, amount: u128, target: AccountId, data: Vec<u8>) -> Result<(), LAssetError>;

    #[ink(message)]
    fn fee_per_million(&self) -> u32;

    #[ink(message)]
    fn set_fee_per_million(&mut self, fee: u32) -> Result<(), LAssetError>;
}

#[ink::trait_definition]
pub trait FlashLoanReceiver {
    #[ink(message)]
    fn on_flash_loan(&mut self, amount: u128, data: Vec<u8>) -> Result<(), FlashCalleeError>;
}
