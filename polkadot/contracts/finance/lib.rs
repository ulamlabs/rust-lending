#![cfg_attr(not(feature = "std"), no_std, no_main)]


#[ink::contract]
pub mod finance {
    use ink::storage::Mapping;

    #[ink(storage)]
    pub struct Finance {
        pub admin: AccountId,
        pub balances: Mapping<AccountId, u128>,
        pub user_balances: Mapping<(AccountId, AccountId), u128>,
        pub tokens: Mapping<AccountId, bool>,
    }

    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    pub enum FinanceError {
        DepositOverflow,
        DepositUserOverflow,
        TokenNotSupported,
        TokenDisabled,
        NothingToWithdraw,
        NothingToWithdrawForUser,
        WithdrawTooMuch,
        WithdrawTooMuchForUser,
        CallerIsNotAdmin,
        #[cfg(any(feature = "std", test, doc))]
        Test(String)
    }

    pub struct EnabledToken(AccountId);
    pub struct WithdrawOnlyToken(AccountId);

    pub trait Token {
        fn id(&self) -> AccountId;
    }
    impl Token for EnabledToken {
        fn id(&self) -> AccountId {
            self.0
        }
    }
    impl Token for WithdrawOnlyToken {
        fn id(&self) -> AccountId {
            self.0
        }
    }

    pub struct NewTokenBalance(u128);
    pub struct NewUserBalance(u128);
    pub struct User(AccountId);
    pub struct AdminCaller();

    impl Finance {
        /// Creates a new flipper smart contract initialized with the given value.
        #[ink(constructor)]
        pub fn new() -> Self {
            let admin = Self::env().caller();
            Finance {
                admin,
                balances: Mapping::default(),
                user_balances: Mapping::default(),
                tokens: Mapping::default(),
            }
        }

        fn enabled_token(&self, token: AccountId) -> Result<EnabledToken, FinanceError> {
            match self.tokens.get(token) {
                Some(v) => match v {
                    true => Ok(EnabledToken(token)),
                    false => Err(FinanceError::TokenDisabled)
                },
                None => {
                    Err(FinanceError::TokenNotSupported)
                }
            }
        }

        fn new_token_balance_after_deposit(&self, token: &EnabledToken, amount: u128) -> Result<NewTokenBalance, FinanceError> {
            if let Some(balance) = self.balances.get(token.0) {
                if let Some(new_balance) = balance.checked_add(amount) {
                    Ok(NewTokenBalance(new_balance))
                } else {
                    Err(FinanceError::DepositOverflow)
                }
            } else {
                Ok(NewTokenBalance(amount))
            }
        }

        fn get_user_balance(&self, token: &impl Token, user: &User) -> Option<u128> {
            self.user_balances.get((token.id(), user.0))
        }

        fn set_user_balance(&mut self, token: &impl Token, user: &User, new_user_balance: NewUserBalance) {
            self.user_balances.insert((token.id(), user.0), &new_user_balance.0);
        }

        fn set_token_balance(&mut self, token: &impl Token, new_balance: NewTokenBalance) {
            self.balances.insert(token.id(), &new_balance.0);
        }

        fn new_user_balance_after_deposit(&self, token: &EnabledToken, user: &User, amount: u128) -> Result<NewUserBalance, FinanceError> {
            if let Some(user_balance) = self.get_user_balance(token, user) {
                if let Some(new_user_balance) = user_balance.checked_add(amount) {
                    Ok(NewUserBalance(new_user_balance))
                } else {
                    Err(FinanceError::DepositUserOverflow)
                }
            } else {
                Ok(NewUserBalance(amount))
            }
        }

        fn caller(&self) -> User {
            let e = self.env();
            User(e.caller())
        }

        fn admin_caller(&self) -> Result<AdminCaller, FinanceError> {
            let user = self.caller();
            if user.0 == self.admin {
                Ok(AdminCaller())
            } else {
                Err(FinanceError::CallerIsNotAdmin)
            }
        }

        fn set_token(&mut self, token: &AccountId, _: &AdminCaller, v: bool) {
            self.tokens.insert(token, &v);
        }

        fn withdraw_only_token(&self, token: AccountId) -> WithdrawOnlyToken {
            WithdrawOnlyToken(token)
        }

        fn new_token_balance_after_withdraw(&self, token: &WithdrawOnlyToken, amount: u128) -> Result<NewTokenBalance, FinanceError> {
            if let Some(balance) = self.balances.get(token.0) {
                if let Some(new_balance) = balance.checked_sub(amount) {
                    Ok(NewTokenBalance(new_balance))
                } else {
                    Err(FinanceError::WithdrawTooMuch)
                }
            } else {
                Err(FinanceError::NothingToWithdraw)
            }
        }

        fn new_user_balance_after_withdraw(&self, token: &WithdrawOnlyToken, user: &User, amount: u128) -> Result<NewUserBalance, FinanceError> {
            if let Some(user_balance) = self.get_user_balance(token, user) {
                if let Some(new_user_balance) = user_balance.checked_sub(amount) {
                    Ok(NewUserBalance(new_user_balance))
                } else {
                    Err(FinanceError::WithdrawTooMuchForUser)
                }
            } else {
                Err(FinanceError::NothingToWithdrawForUser)
            }
        }


        #[ink(message)]
        pub fn deposit(&mut self, token: AccountId, amount: u128) -> Result<(), FinanceError> {
            let user = &self.caller();
            let token = &self.enabled_token(token)?;
            let new_user_balance = self.new_user_balance_after_deposit(token, user, amount)?;
            let new_balance = self.new_token_balance_after_deposit(token, amount)?;

            self.set_user_balance(token, user, new_user_balance);
            self.set_token_balance(token, new_balance);

            Ok(())
        }

        #[ink(message)]
        pub fn withdraw(&mut self, token: AccountId, amount: u128) -> Result<(), FinanceError> {
            let user = &self.caller();
            let token = &self.withdraw_only_token(token);
            let new_balance = self.new_token_balance_after_withdraw(token, amount)?;
            let new_user_balance = self.new_user_balance_after_withdraw(token, user, amount)?;

            self.set_token_balance(token, new_balance);
            self.set_user_balance(token, user, new_user_balance);

            Ok(())
        }

        #[ink(message)]
        pub fn disable(&mut self, token: AccountId) -> Result<(), FinanceError> { 
            let admin = &self.admin_caller()?;

            self.set_token(&token, admin, false);
            Ok(())
        }

        #[ink(message)]
        pub fn enable(&mut self, token: AccountId) -> Result<(), FinanceError> { 
            let admin = &self.admin_caller()?;

            self.set_token(&token, admin, true);
            Ok(())
        }

        #[ink(message)]
        pub fn balance(&self, token: AccountId) -> u128 {
            if let Some(balance) = self.balances.get(token) {
                balance
            } else {
                0
            }
        }

        #[ink(message)]
        pub fn user_balance(&self, token: AccountId, user: AccountId) -> u128 {
            if let Some(user_balance) = self.user_balances.get((token, user)) {
                user_balance
            } else {
                0
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn accounts(
        ) -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> {
            ink::env::test::default_accounts::<ink::env::DefaultEnvironment>()
        }

        fn set_caller(caller: AccountId) {
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(caller);
        }

        fn e(m: &'static str) -> Result<(), FinanceError> {
            Err(FinanceError::Test(String::from(m)))
        }

        fn _run() -> Result<(), FinanceError> {
            let callers = accounts();
            let admin = callers.alice;
            let user = callers.django;
            let eth = callers.eve;
            let btc = callers.bob;


            set_caller(admin);
            let mut finance = Finance::new();
            
            match finance.deposit(btc, 100) {
                Err(FinanceError::TokenNotSupported) => Ok(()),
                _ => e("Deposit should fail if token is not supported"),
            }?;
            
            set_caller(user);
            match finance.disable(btc) {
                Err(FinanceError::CallerIsNotAdmin) => Ok(()),
                _ => e("Disable should fail if caller is not admin"),
            }?;

            set_caller(admin);
            finance.disable(btc)?;

            match finance.deposit(btc, 100) {
                Err(FinanceError::TokenDisabled) => Ok(()),
                _ => e("Deposit should fail if token is disabled"),
            }?;

            set_caller(user);
            match finance.enable(btc) {
                Err(FinanceError::CallerIsNotAdmin) => Ok(()),
                _ => e("Enable should fail if caller is not admin"),
            }?;

            set_caller(admin);
            finance.enable(btc)?;

            set_caller(user);
            finance.deposit(btc, u128::MAX)?;

            match finance.deposit(btc, 1) {
                Err(FinanceError::DepositUserOverflow) => Ok(()),
                _ => e("Deposit should fail if integer overflow occurs, while increasing user balance"),
            }?;

            set_caller(admin);
            match finance.deposit(btc, 1) {
                Err(FinanceError::DepositOverflow) => Ok(()),
                _ => e("Deposit should fail if integer overflow occurs, while increasing token balance"),
            }?;

            match finance.withdraw(eth, 0) {
                Err(FinanceError::NothingToWithdraw) => Ok(()),
                _ => e("Withdraw should fail if token has no balance"),
            }?;

            match finance.withdraw(btc, 0) {
                Err(FinanceError::NothingToWithdrawForUser) => Ok(()),
                _ => e("Withdraw should fail if user has no balance"),
            }?;
            
            set_caller(user);
            finance.withdraw(btc, u128::MAX)?;

            match finance.withdraw(btc, u128::MAX) {
                Err(FinanceError::WithdrawTooMuch) => Ok(()),
                _ => e("Withdraw should fail if token has not enough balance"),
            }?;

            set_caller(admin);
            finance.deposit(btc, 1)?;
            finance.deposit(btc, 0)?;

            set_caller(user);
            match finance.withdraw(btc, 1) {
                Err(FinanceError::WithdrawTooMuchForUser) => Ok(()),
                _ => e("Withdraw should fail if user has not enough balance")
            }?;



            Ok(())
        }

        #[ink::test]
        fn run() -> Result<(), ink::env::Error> {
            if let Err(e) = _run() {
                eprintln!("{:?}", e);
                Err(ink::env::Error::CallRuntimeFailed)
            } else {
                Ok(())
            }
        }
        
    }
}