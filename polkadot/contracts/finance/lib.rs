#![cfg_attr(not(feature = "std"), no_std, no_main)]


#[ink::contract]
pub mod finance {
    use ink::storage::Mapping;

    #[ink(storage)]
    #[derive(Default)]
    pub struct Finance {
        pub balances: Mapping<AccountId, u128>,
        pub user_balances: Mapping<(AccountId, AccountId), u128>,
        pub tokens: Mapping<AccountId, bool>,
    }

    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    pub enum Error {
        DepositOverflow,
        DepositUserOverflow,
        TokenNotSupported,
        TokenDisabled,
    }

    pub struct EnabledToken(AccountId);

    pub struct NewTokenBalance(u128);
    pub struct NewUserBalance(u128);
    pub struct User(AccountId);

    impl Finance {
        /// Creates a new flipper smart contract initialized with the given value.
        #[ink(constructor)]
        pub fn new() -> Self {
            Self::default()
        }

        fn enabled_token(&self, token: AccountId) -> Result<EnabledToken, Error> {
            match self.tokens.get(token) {
                Some(v) => match v {
                    true => Ok(EnabledToken(token)),
                    false => Err(Error::TokenDisabled)
                },
                None => {
                    Err(Error::TokenNotSupported)
                }
            }
        }

        fn new_token_balance_after_deposit(&self, token: &EnabledToken, amount: u128) -> Result<NewTokenBalance, Error> {
            if let Some(balance) = self.balances.get(token.0) {
                if let Some(new_balance) = balance.checked_add(amount) {
                    Ok(NewTokenBalance(new_balance))
                } else {
                    Err(Error::DepositOverflow)
                }
            } else {
                Ok(NewTokenBalance(amount))
            }
        }

        fn get_user_balance(&self, token: &EnabledToken, user: &User) -> Option<u128> {
            self.user_balances.get((token.0, user.0))
        }

        fn set_user_balance(&mut self, token: &EnabledToken, user: &User, new_user_balance: NewUserBalance) {
            self.user_balances.insert((token.0, user.0), &new_user_balance.0);
        }

        fn set_token_balance(&mut self, token: &EnabledToken, new_balance: NewTokenBalance) {
            self.balances.insert(token.0, &new_balance.0);
        }

        fn new_user_balance_after_deposit(&self, token: &EnabledToken, user: &User, amount: u128) -> Result<NewUserBalance, Error> {
            if let Some(user_balance) = self.get_user_balance(token, user) {
                if let Some(new_user_balance) = user_balance.checked_add(amount) {
                    Ok(NewUserBalance(new_user_balance))
                } else {
                    Err(Error::DepositUserOverflow)
                }
            } else {
                Ok(NewUserBalance(amount))
            }
        }

        fn caller(&self) -> User {
            User(self.env().caller())
        }


        #[ink(message)]
        pub fn deposit(&mut self, token: AccountId, amount: u128) -> Result<(), Error> {
            let user = &self.caller();
            let token = &self.enabled_token(token)?;
            let new_user_balance = self.new_user_balance_after_deposit(token, user, amount)?;
            let new_balance = self.new_token_balance_after_deposit(token, amount)?;

            self.set_user_balance(token, user, new_user_balance);
            self.set_token_balance(token, new_balance);

            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn deposit_fails_if_token_not_enabled() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
        }
    }
}