pub use ink::primitives::AccountId;

pub use crate::finance2::{LAssetContract, BALANCES, BTC_ADDRESS, CALLER, CALLEE, ETH_ADDRESS, L_BTC, L_ETH, L_USDC, TRANSFER_ERROR, USDC_ADDRESS};
pub use crate::errors::{LAssetError, TakeCashError};
pub use crate::structs::{AssetParams, AssetPool, LAsset};
pub use traits::psp22::PSP22;

pub fn setup_call(caller: AccountId, callee: AccountId, value: u128, timestamp: u64) {
    unsafe {
        CALLER = Some(caller);
        CALLEE = Some(callee);
    }
    ink::env::test::set_caller::<ink::env::DefaultEnvironment>(caller);
    ink::env::test::set_callee::<ink::env::DefaultEnvironment>(callee);
    ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(timestamp);
    ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(value);
}

fn e<T: std::fmt::Debug>(m: &str, r: T) -> Result<(), String> {
    Err(format!("{}. Got: {:?}", m, r))
}


#[ink::test]
fn default_works() {
    let btc = AccountId::from(BTC_ADDRESS);
    let usdc = AccountId::from(USDC_ADDRESS);
    let eth = AccountId::from(ETH_ADDRESS);
    let admin = AccountId::from([0x4; 32]);

    let alice = AccountId::from([0x8; 32]);
    let bob = AccountId::from([0x9; 32]);
    let timestamp = 0;

    unsafe {
        BALANCES = Some(std::collections::HashMap::new());

        setup_call(admin, btc, 0, 0);
        L_BTC = Some(LAssetContract::new(btc, usdc, 1));
        setup_call(admin, usdc, 0, 0);
        L_USDC = Some(LAssetContract::new(usdc, eth, 1));
        setup_call(admin, eth, 0, 0);
        L_ETH = Some(LAssetContract::new(eth, btc, 1));
    }
    let (transfer_error, balances, l_btc, l_usdc, l_eth) = unsafe {
        (&mut TRANSFER_ERROR, BALANCES.as_mut().unwrap(), L_BTC.as_mut().unwrap(), L_USDC.as_mut().unwrap(), L_ETH.as_mut().unwrap())
    };
    {
        l_btc.price = 1;
        l_usdc.price = 1;
        l_eth.price = 1;
    }
    {
        setup_call(alice, btc, 0, timestamp);
        match l_btc.set_price(0, 0) {
            Err(LAssetError::SetPriceUnathorized) => Ok(()),
            r => e("Set price should fail if unauthorized", r),
        }.unwrap();
    }
    {
        setup_call(alice, eth, 0, timestamp);
        let params = AssetParams {
            standard_rate: 0,
            standard_min_rate: 0,
            emergency_rate: 0,
            emergency_max_rate: 0,
            initial_margin: 0,
            maintenance_margin: 0,
            initial_haircut: u128::MAX,
            maintenance_haircut: u128::MAX,
            mint_fee: 0,
            borrow_fee: 0,
            take_cash_fee: 0,
            liquidation_reward: 0,
        };
        match l_eth.set_params(params) {
            Err(LAssetError::SetParamsUnathorized) => Ok(()),
            r => e("Set params should fail if unauthorized", r),
        }.unwrap();
    }
    {
        setup_call(alice, btc, 0, timestamp);
        match l_btc.deposit(0) {
            Err(LAssetError::FirstDepositRequiresGasCollateral) => Ok(()),
            r => e("First deposit should fail without gas collateral", r),
        }.unwrap();
    }
    {
        setup_call(alice, btc, 0, timestamp);
        match l_btc.repay(alice, 0) {
            Err(LAssetError::RepayWithoutBorrow) => Ok(()),
            r => e("Repay without borrow should fail", r),
        }.unwrap();
    }
    {
        setup_call(alice, btc, 1, timestamp);
        l_btc.deposit(0).unwrap();
    }
    {
        setup_call(alice, btc, 0, timestamp);
        match l_btc.borrow(0) {
            Err(LAssetError::BorrowWhileDepositingNotAllowed) => Ok(()),
            r => e("Borrow while depositing should fail", r),
        }.unwrap();
    }
    {
        setup_call(alice, btc, 0, timestamp);
        match l_btc.withdraw(u128::MAX) {
            Err(LAssetError::WithdrawOverflow) => Ok(()),
            r => e("Withdraw should fail on overflow", r),
        }.unwrap();
    }
    {
        setup_call(alice, btc, 0, timestamp);
        match l_btc.deposit(1) {
            Err(LAssetError::DepositTransferFailed(_)) => Ok(()),
            r => e("Deposit should fail if caller has insufficient balance", r),
        }.unwrap();
    }
    {
        balances.insert((btc, alice), u128::MAX);
        setup_call(alice, btc, 0, timestamp);
        l_btc.deposit(u128::MAX).unwrap();
    }
    {
        balances.insert((btc, alice), 1);
        setup_call(alice, btc, 0, timestamp);
        match l_btc.deposit(1) {
            Err(LAssetError::DepositOverflow) => Ok(()),
            r => e("Deposit should fail on overflow", r),
        }.unwrap();
    }
    {
        *transfer_error = true;
        setup_call(alice, btc, 0, timestamp);
        match l_btc.withdraw(0) {
            Err(LAssetError::WithdrawTransferFailed(_)) => Ok(()),
            r => e("Withdraw should fail if transfer fails", r),
        }.unwrap();
    }
    {
        setup_call(alice, btc, 0, timestamp);
        l_btc.withdraw(u128::MAX).unwrap();
    }
    {
        setup_call(bob, btc, 0, timestamp);
        match l_btc.withdraw(0) {
            Err(LAssetError::WithdrawWithoutDeposit) => Ok(()),
            r => e("Withdraw without deposit should fail", r),
        }.unwrap();
    }
    {
        setup_call(alice, btc, 0, timestamp);
        match l_btc.borrow(0) {
            Err(LAssetError::FirstBorrowRequiresGasCollateral) => Ok(()),
            r => e("First borrow should fail without gas collateral", r),
        }.unwrap();
    }
    {
        setup_call(alice, btc, 1, timestamp);
        match l_btc.borrow(u128::MAX) {
            Err(LAssetError::BorrowOverflow) => Ok(()),
            r => e("Borrow should fail on overflow", r),
        }.unwrap();
    }
    {
        balances.insert((btc, alice), 0);
        setup_call(alice, btc, 0, timestamp);
        match l_btc.mint(1) {
            Err(LAssetError::MintTransferFailed(_)) => Ok(()),
            r => e("Mint should fail if transfer fails", r),
        }.unwrap();
    }
    {
        l_btc.params.mint_fee = u128::MAX;
        setup_call(alice, btc, 0, timestamp);
        match l_btc.mint(u128::MAX) {
            Err(LAssetError::MintFeeOverflow) => Ok(()),
            r => e("Mint should fail on fee overflow", r),
        }.unwrap();
        l_btc.params.mint_fee = 0;
    }
    {
        balances.insert((btc, alice), 3);
        setup_call(alice, btc, 0, timestamp);
        l_btc.mint(3).unwrap();
    }
    {
        balances.insert((usdc, alice), 4);
        setup_call(alice, usdc, 1, timestamp);
        l_usdc.deposit(4).unwrap();
    }
    {
        setup_call(alice, btc, 1, timestamp);
        l_btc.borrow(1).unwrap();
    }
    {
        *transfer_error = true;
        setup_call(alice, btc, 0, timestamp);
        match l_btc.borrow(0) {
            Err(LAssetError::BorrowTransferFailed(_)) => Ok(()),
            r => e("Borrow should fail if transfer fails", r),
        }.unwrap();
    }
    {
        setup_call(alice, btc, 0, timestamp);
        match l_btc.borrow(2) {
            Err(LAssetError::CollateralValueTooLowAfterBorrow) => Ok(()),
            r => e("Borrow should fail if collateral value too low", r),
        }.unwrap();
    }
    {
        l_btc.params.borrow_fee = u128::MAX;
        setup_call(alice, btc, 0, timestamp);
        match l_btc.borrow(u128::MAX) {
            Err(LAssetError::BorrowFeeOverflow) => Ok(()),
            r => e("Borrow should fail on fee overflow", r),
        }.unwrap();
        l_btc.params.borrow_fee = 0;
    }
    {
        setup_call(alice, btc, 0, timestamp);
        match l_btc.burn(3) {
            Err(LAssetError::BurnTooMuch) => Ok(()),
            r => e("Burn should fail if burn too much", r),
        }.unwrap();
    }
    {
        *transfer_error = true;
        setup_call(alice, btc, 0, timestamp);
        match l_btc.burn(1) {
            Err(LAssetError::BurnTransferFailed(_)) => Ok(()),
            r => e("Burn should fail if transfer fails", r),
        }.unwrap();
    }
    {
        setup_call(alice, usdc, 0, timestamp);
        match l_usdc.withdraw(2) {
            Err(LAssetError::CollateralValueTooLowAfterWithdraw) => Ok(()),
            r => e("Withdraw should fail if collateral value too low", r),
        }.unwrap();
    }
    {
        setup_call(alice, btc, 1, timestamp);
        match l_btc.deposit(1) {
            Err(LAssetError::DepositWhileBorrowingNotAllowed) => Ok(()),
            r => e("Deposit while borrowing should fail", r),
        }.unwrap();
    }
    {
        balances.insert((btc, alice), u128::MAX);
        setup_call(alice, btc, 0, timestamp);
        match l_btc.mint(u128::MAX) {
            Err(LAssetError::MintOverflow) => Ok(()),
            r => e("Mint liquidity should fail on overflow", r),
        }.unwrap();
    }
    {
        setup_call(alice, btc, 0, timestamp);
        match l_btc.burn(u128::MAX) {
            Err(LAssetError::BurnOverflow) => Ok(()),
            r => e("Burn should fail on overflow", r),
        }.unwrap();
    }
    {
        balances.insert((btc, alice), 0);
        setup_call(alice, btc, 0, timestamp);
        match l_btc.repay(alice, 1) {
            Err(LAssetError::RepayTransferFailed(_)) => Ok(()),
            r => e("Repay should fail if transfer fails", r),
        }.unwrap();
    }
    {
        setup_call(alice, btc, 0, timestamp);
        match l_btc.deposit_cash(alice, 1) {
            Err(LAssetError::DepositCashTransferFailed(_)) => Ok(()),
            r => e("Deposit cash should fail if transfer fails", r),
        }.unwrap();
    }
    {
        balances.insert((btc, alice), u128::MAX);
        setup_call(alice, btc, 0, timestamp);
        l_btc.deposit_cash(alice, u128::MAX).unwrap();
    }
    {
        balances.insert((btc, alice), 1);
        setup_call(alice, btc, 0, timestamp);
        match l_btc.deposit_cash(alice, 1) {
            Err(LAssetError::DepositCashOverflow) => Ok(()),
            r => e("Deposit cash should fail on overflow", r),
        }.unwrap();
    }
    {
        *transfer_error = true;
        setup_call(alice, btc, 0, timestamp);
        match l_btc.withdraw_cash() {
            Err(LAssetError::WithdrawCashTransferFailed(_)) => Ok(()),
            r => e("Withdraw cash should fail if transfer fails", r),
        }.unwrap();
    }
    {
        balances.insert((btc, alice), u128::MAX);
        setup_call(alice, btc, 0, timestamp);
        l_btc.deposit_cash(alice, u128::MAX).unwrap();
    }
    {
        balances.insert((btc, alice), 1);
        setup_call(alice, btc, 0, timestamp);
        match l_btc.repay(alice, 1) {
            Err(LAssetError::RepayCashOverflow) => Ok(()),
            r => e("Repay cash should fail on overflow", r),
        }.unwrap();
    }
    {
        setup_call(alice, btc, 0, timestamp);
        match l_btc.liquidate(bob) {
            Err(LAssetError::LiquidateForNothing) => Ok(()),
            r => e("Liquidate should fail if nothing to liquidate", r),
        }.unwrap();
    }
    {
        setup_call(alice, usdc, 0, timestamp);
        match l_usdc.liquidate(alice) {
            Err(LAssetError::LiquidateTooEarly) => Ok(()),
            r => e("Liquidate should fail if too early", r),
        }.unwrap();
    }
    {
        balances.insert((eth, alice), 1);
        setup_call(alice, eth, 0, timestamp);
        l_eth.mint(1).unwrap();

        setup_call(alice, eth, 1, timestamp);
        l_eth.borrow(1).unwrap();

        *transfer_error = true;
        l_usdc.price_scaler = 2;
        setup_call(alice, usdc, 0, timestamp);
        match l_usdc.liquidate(alice) {
            Err(LAssetError::LiquidateTransferFailed(_)) => Ok(()),
            r => e("Liquidate should fail if transfer fails", r),
        }.unwrap();
        l_usdc.price_scaler = 1;
    }
    {
        l_eth.params.maintenance_margin = u128::MAX;
        l_btc.params.maintenance_margin = u128::MAX;
        setup_call(alice, usdc, 0, timestamp);
        match l_usdc.liquidate(alice) {
            Err(LAssetError::LiquidateTooMuch) => Ok(()),
            r => e("Liquidate should fail if too much", r),
        }.unwrap();
    }
    {
        setup_call(alice, btc, 0, timestamp);
        match l_btc.take_cash(0, alice) {
            Err(TakeCashError::Unauthorized) => Ok(()),
            r => e("Take cash should fail if unauthorized", r),
        }.unwrap();
    }
    {
        l_btc.params.take_cash_fee = u128::MAX;
        setup_call(admin, btc, 0, timestamp);
        l_btc.take_cash(u128::MAX / 2, admin).unwrap();
        match l_btc.take_cash(u128::MAX / 2, admin) {
            Err(TakeCashError::Overflow) => Ok(()),
            r => e("Take cash should fail on overflow", r),
        }.unwrap();
    }
    {
        *transfer_error = true;
        l_btc.params.take_cash_fee = 0;
        setup_call(admin, btc, 0, timestamp);
        match l_btc.take_cash(0, admin) {
            Err(TakeCashError::Transfer(_)) => Ok(()),
            r => e("Take cash should fail if transfer fails", r),
        }.unwrap();
    }
}