
mod tests {
    use crate::finance::Finance;
    use ink_primitives::AccountId;
    use traits::errors::FinanceError;
    use traits::FinanceTrait;

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
        let oracle = callers.frank;
        let user = callers.django;
        let eth = callers.eve;
        let btc = callers.bob;


        set_caller(admin);
        let mut finance = Finance::new(oracle);
        
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
        match finance.enable(btc, btc) {
            Err(FinanceError::CallerIsNotAdmin) => Ok(()),
            _ => e("Enable should fail if caller is not admin"),
        }?;

        set_caller(admin);
        finance.enable(btc, btc)?;

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