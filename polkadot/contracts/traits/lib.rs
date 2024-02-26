#![cfg_attr(not(feature = "std"), no_std, no_main)]

pub mod errors;
pub mod psp22;

use ink::primitives::AccountId;
use ink::prelude::vec::Vec;
pub use errors::{LAssetError, FlashCalleeError};

#[ink::trait_definition]
pub trait FlashLoanPool {
    /// Authorized method: only the flash loan contract can call it to receive funds
    ///
    /// Args:
    /// - amount: the amount of tokens to borrow
    /// - target: the address of the contract that will receive the funds
    ///
    /// Returns:
    /// - the address of the token that was borrowed
    #[ink(message)]
    fn take_cash(&mut self, amount: u128, target: AccountId) -> Result<AccountId, LAssetError>;

    #[ink(message)]
    fn underlying_token(&self) -> AccountId;
}

#[ink::trait_definition]
pub trait FlashLoanContract { 
    #[ink(message)]
    fn flash_loan(&mut self, target: AccountId, pool_address: AccountId, amount: u128, data: Vec<u8>) -> Result<(), LAssetError>;

    #[ink(message)]
    fn fee_per_million(&self) -> u32;

    #[ink(message)]
    fn set_fee_per_million(&mut self, fee: u32) -> Result<(), LAssetError>;
}

#[ink::trait_definition]
pub trait FlashLoanReceiver {
    /// Interface for the flash loan receiver contract
    /// The recipient must increase allowance in the calling contract by amount + fee
    /// This interface is based on EIP-3156 (https://eips.ethereum.org/EIPS/eip-3156)
    #[ink(message)]
    fn on_flash_loan(&mut self, initiator: AccountId, token: AccountId, amount: u128, fee: u128, data: Vec<u8>) -> Result<(), FlashCalleeError>;
}
