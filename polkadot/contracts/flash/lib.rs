#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod flash {
    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    pub struct Flash {
        pub fee_admin: AccountId,
        pub fee_per_million: u32,
    }
    use ink::contract_ref;
<<<<<<< HEAD
    use ink::prelude::vec::Vec;
=======
>>>>>>> 7bd404f (feat: Flash Loan Contract)
    use traits::{FlashLoanPool, FlashLoanContract, FlashLoanReceiver};
    use traits::psp22::PSP22;
    use traits::errors::LAssetError;

    impl Flash {
        /// Constructor that initializes the `bool` value to the given `init_value`.
        #[ink(constructor)]
        pub fn new(fee_per_million: u32) -> Self {
            Self { 
                fee_admin: Self::env().caller(),
                fee_per_million,
            }
        }

        fn calculate_fee(&self, amount: u128) -> u128 {
            amount.wrapping_mul(self.fee_per_million as u128).wrapping_div(1_000_000)
        }
    }

    impl FlashLoanContract for Flash {
        /// Borrow tokens from a lending pool
        #[ink(message)]
        fn flash_loan(&mut self, target_address: AccountId, pool_address: AccountId, amount: u128, data: Vec<u8>) -> Result<(), LAssetError>{
            let mut pool: contract_ref!(FlashLoanPool) = pool_address.into();
            let fee = self.calculate_fee(amount);

            // 1. Call the `take_cash` method of the pool to borrow the tokens
            let mut underlying_token: contract_ref!(PSP22) = pool.take_cash(amount, target_address)?.into();

            // 2. Call the `on_flash_loan` method of the target
            let mut target: contract_ref!(FlashLoanReceiver) = target_address.into();
            target.on_flash_loan(self.env().caller(), *underlying_token.as_ref(), amount, fee, data).map_err(LAssetError::FlashLoanFailed)?;

            // 3. Return the tokens to the pool
            let new_amount = amount.saturating_add(fee);
            underlying_token.transfer_from(target_address, pool_address, new_amount, Vec::new())
                .map_err(LAssetError::FlashLoanTransferFailed)?;
            
            Ok(())
        }

        /// Get the fee per million
        #[ink(message)]
        fn fee_per_million(&self) -> u32 {
            self.fee_per_million
        }

        /// Set the fee per million
        /// Only the owner can call this method
        #[ink(message)]
        fn set_fee_per_million(&mut self, fee: u32) -> Result<(), LAssetError>  {
            if self.env().caller() != self.fee_admin {
                return Err(LAssetError::Unathorized);
            }
            self.fee_per_million = fee;

            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        /// We test if the default constructor does its job.
        #[ink::test]
        fn fee_per_million_works() {
            let flash = Flash::new(1234);
            assert_eq!(flash.fee_per_million(), 1234);
        }
    }
}
