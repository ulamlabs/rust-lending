#![cfg_attr(not(feature = "std"), no_std)]

#[ink::contract]
pub mod finance {
    use ink::storage::Mapping;

    #[ink(storage)]
    #[derive(Default)]
    pub struct Finance {
        pub balances: Mapping<AccountId, u64>,
    }

    impl Finance {
        /// Creates a new flipper smart contract initialized with the given value.
        #[ink(constructor)]
        pub fn new() -> Self {
            Self::default()
        }

        #[ink(message)]
        pub fn set_balance(&mut self, account: AccountId, balance: u64) {
            // check if account is the sender
            assert_eq!(account, self.env().caller(), "Not authorized");

            self.balances.insert(account, &balance);
        }

        #[ink(message)]
        pub fn get_balance(&self, account: AccountId) -> u64 {
            self.balances.get(&account).unwrap_or(0)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn it_works() {
            let mut finance = Finance::new();
            finance.set_balance(AccountId::from([0x01; 32]), 100);
            
            assert_eq!(finance.get_balance(AccountId::from([0x01; 32])), 100);
        }
    }
}