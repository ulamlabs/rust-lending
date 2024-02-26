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
        fn flash_loan(&mut self, pool_address: AccountId, amount: u128, target: AccountId, data: Vec<u8>) -> Result<(), LAssetError>{
            let mut pool: contract_ref!(FlashLoanPool) = pool_address.into();
            let mut underlying_token: contract_ref!(PSP22) = pool.underlying_token().into();

            // 1. Call the `take_cash` method of the pool to borrow the tokens
            pool.take_cash(amount)?;

            // 2. Call the `transfer` method of the underlying token to send the tokens to the target
            underlying_token.transfer(target, amount, vec![]).map_err(LAssetError::FlashLoanTransferFailed)?;

            // 3. Call the `on_flash_loan` method of the target
            let mut target: contract_ref!(FlashLoanReceiver) = target.into();
            target.on_flash_loan(amount, data).map_err(LAssetError::FlashLoanFailed)?;

            // 4. Return the tokens to the pool
            let new_amount = amount.saturating_add(self.calculate_fee(amount));
            underlying_token.transfer(pool_address, new_amount, Vec::new()).map_err(LAssetError::FlashLoanTransferFailed)?;
            
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
                return Err(LAssetError::CallerIsNotAdmin);
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
