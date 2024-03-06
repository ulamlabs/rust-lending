use ink::primitives::AccountId;
use traits::FlashLoanPoolError;
use traits::FlashLoanPool;

use crate::finance2::*;
use crate::errors::LAssetError;

fn setup_call(caller: AccountId, callee: AccountId, value: u128, timestamp: u64) {
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
    let l_btc = AccountId::from([0x1; 32]);
    let l_usdc = AccountId::from([0x2; 32]);
    let l_eth = AccountId::from([0x3; 32]);
    let admin = AccountId::from([0x4; 32]);
    let btc = AccountId::from([0x5; 32]);
    let usdc = AccountId::from([0x6; 32]);
    let eth = AccountId::from([0x7; 32]);

    let alice = AccountId::from([0x8; 32]);
    let bob = AccountId::from([0x9; 32]);
    let timestamp = 0;

    unsafe {
        BALANCES = Some(std::collections::HashMap::new());

        setup_call(admin, l_btc, 0, 0);
        L_BTC = Some(LAssetContract::new(btc, l_usdc, 1));
        setup_call(admin, l_usdc, 0, 0);
        L_USDC = Some(LAssetContract::new(usdc, l_eth, 1));
        setup_call(admin, l_eth, 0, 0);
        L_ETH = Some(LAssetContract::new(eth, l_btc, 1));
    }
    let (transfer_error, balances, btc_app, usdc_app, eth_app) = unsafe {
        (&mut TRANSFER_ERROR, BALANCES.as_mut().unwrap(), L_BTC.as_mut().unwrap(), L_USDC.as_mut().unwrap(), L_ETH.as_mut().unwrap())
    };
    {
        setup_call(alice, l_btc, 0, timestamp);
        match btc_app.set_price(0, 0) {
            Err(LAssetError::SetPriceUnathorized) => Ok(()),
            r => e("Set price should fail if unauthorized", r),
        }.unwrap();
    }
    {
        setup_call(alice, l_eth, 0, timestamp);
        match eth_app.set_params(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0) {
            Err(LAssetError::SetParamsUnathorized) => Ok(()),
            r => e("Set params should fail if unauthorized", r),
        }.unwrap();
    }
    {
        setup_call(alice, l_btc, 0, timestamp);
        match btc_app.deposit(0) {
            Err(LAssetError::FirstDepositRequiresGasCollateral) => Ok(()),
            r => e("First deposit should fail without gas collateral", r),
        }.unwrap();
    }
    {
        setup_call(alice, l_btc, 0, timestamp);
        match btc_app.repay(alice, 0) {
            Err(LAssetError::RepayWithoutBorrow) => Ok(()),
            r => e("Repay without borrow should fail", r),
        }.unwrap();
    }
    {
        setup_call(alice, l_btc, 1, timestamp);
        btc_app.deposit(0).unwrap();
    }
    {
        setup_call(alice, l_btc, 0, timestamp);
        match btc_app.borrow(0) {
            Err(LAssetError::BorrowWhileDepositingNotAllowed) => Ok(()),
            r => e("Borrow while depositing should fail", r),
        }.unwrap();
    }
    {
        setup_call(alice, l_btc, 0, timestamp);
        match btc_app.withdraw(u128::MAX) {
            Err(LAssetError::WithdrawOverflow) => Ok(()),
            r => e("Withdraw should fail on overflow", r),
        }.unwrap();
    }
    {
        setup_call(alice, l_btc, 0, timestamp);
        match btc_app.deposit(1) {
            Err(LAssetError::DepositTransferFailed(_)) => Ok(()),
            r => e("Deposit should fail if caller has insufficient balance", r),
        }.unwrap();
    }
    {
        balances.insert((btc, alice), u128::MAX);
        setup_call(alice, l_btc, 0, timestamp);
        btc_app.deposit(u128::MAX).unwrap();
    }
    {
        balances.insert((btc, alice), 1);
        setup_call(alice, l_btc, 0, timestamp);
        match btc_app.deposit(1) {
            Err(LAssetError::DepositOverflow) => Ok(()),
            r => e("Deposit should fail on overflow", r),
        }.unwrap();
    }
    {
        *transfer_error = true;
        setup_call(alice, l_btc, 0, timestamp);
        match btc_app.withdraw(0) {
            Err(LAssetError::WithdrawTransferFailed(_)) => Ok(()),
            r => e("Withdraw should fail if transfer fails", r),
        }.unwrap();
    }
    {
        setup_call(alice, l_btc, 0, timestamp);
        btc_app.withdraw(u128::MAX).unwrap();
    }
    {
        setup_call(bob, l_btc, 0, timestamp);
        match btc_app.withdraw(0) {
            Err(LAssetError::WithdrawWithoutDeposit) => Ok(()),
            r => e("Withdraw without deposit should fail", r),
        }.unwrap();
    }
    {
        setup_call(alice, l_btc, 0, timestamp);
        match btc_app.borrow(0) {
            Err(LAssetError::FirstBorrowRequiresGasCollateral) => Ok(()),
            r => e("First borrow should fail without gas collateral", r),
        }.unwrap();
    }
    {
        setup_call(alice, l_btc, 1, timestamp);
        match btc_app.borrow(u128::MAX) {
            Err(LAssetError::BorrowOverflow) => Ok(()),
            r => e("Borrow should fail on overflow", r),
        }.unwrap();
    }
    {
        balances.insert((btc, alice), 0);
        setup_call(alice, l_btc, 0, timestamp);
        match btc_app.mint(1) {
            Err(LAssetError::MintTransferFailed(_)) => Ok(()),
            r => e("Mint should fail if transfer fails", r),
        }.unwrap();
    }
    {
        btc_app.mint_fee = u128::MAX;
        setup_call(alice, l_btc, 0, timestamp);
        match btc_app.mint(u128::MAX) {
            Err(LAssetError::MintFeeOverflow) => Ok(()),
            r => e("Mint should fail on fee overflow", r),
        }.unwrap();
        btc_app.mint_fee = 0;
    }
    {
        balances.insert((btc, alice), 3);
        setup_call(alice, l_btc, 0, timestamp);
        btc_app.mint(3).unwrap();
    }
    {
        balances.insert((usdc, alice), 4);
        setup_call(alice, l_usdc, 1, timestamp);
        usdc_app.deposit(4).unwrap();
    }
    {
        setup_call(alice, l_btc, 1, timestamp);
        btc_app.borrow(1).unwrap();
    }
    {
        *transfer_error = true;
        setup_call(alice, l_btc, 0, timestamp);
        match btc_app.borrow(0) {
            Err(LAssetError::BorrowTransferFailed(_)) => Ok(()),
            r => e("Borrow should fail if transfer fails", r),
        }.unwrap();
    }
    {
        setup_call(alice, l_btc, 0, timestamp);
        match btc_app.borrow(2) {
            Err(LAssetError::CollateralValueTooLowAfterBorrow) => Ok(()),
            r => e("Borrow should fail if collateral value too low", r),
        }.unwrap();
    }
    {
        btc_app.borrow_fee = u128::MAX;
        setup_call(alice, l_btc, 0, timestamp);
        match btc_app.borrow(u128::MAX) {
            Err(LAssetError::BorrowFeeOverflow) => Ok(()),
            r => e("Borrow should fail on fee overflow", r),
        }.unwrap();
        btc_app.borrow_fee = 0;
    }
    {
        setup_call(alice, l_btc, 0, timestamp);
        match btc_app.burn(3) {
            Err(LAssetError::BurnTooMuch) => Ok(()),
            r => e("Burn should fail if burn too much", r),
        }.unwrap();
    }
    {
        *transfer_error = true;
        setup_call(alice, l_btc, 0, timestamp);
        match btc_app.burn(1) {
            Err(LAssetError::BurnTransferFailed(_)) => Ok(()),
            r => e("Burn should fail if transfer fails", r),
        }.unwrap();
    }
    {
        setup_call(alice, l_usdc, 0, timestamp);
        match usdc_app.withdraw(2) {
            Err(LAssetError::CollateralValueTooLowAfterWithdraw) => Ok(()),
            r => e("Withdraw should fail if collateral value too low", r),
        }.unwrap();
    }
    {
        setup_call(alice, l_btc, 1, timestamp);
        match btc_app.deposit(1) {
            Err(LAssetError::DepositWhileBorrowingNotAllowed) => Ok(()),
            r => e("Deposit while borrowing should fail", r),
        }.unwrap();
    }
    {
        balances.insert((btc, alice), u128::MAX);
        setup_call(alice, l_btc, 0, timestamp);
        match btc_app.mint(u128::MAX) {
            Err(LAssetError::MintOverflow) => Ok(()),
            r => e("Mint liquidity should fail on overflow", r),
        }.unwrap();
    }
    {
        setup_call(alice, l_btc, 0, timestamp);
        match btc_app.burn(u128::MAX) {
            Err(LAssetError::BurnOverflow) => Ok(()),
            r => e("Burn should fail on overflow", r),
        }.unwrap();
    }
    {
        balances.insert((btc, alice), 0);
        setup_call(alice, l_btc, 0, timestamp);
        match btc_app.repay(alice, 1) {
            Err(LAssetError::RepayTransferFailed(_)) => Ok(()),
            r => e("Repay should fail if transfer fails", r),
        }.unwrap();
    }
    {
        setup_call(alice, l_btc, 0, timestamp);
        match btc_app.deposit_cash(alice, 1) {
            Err(LAssetError::DepositCashTransferFailed(_)) => Ok(()),
            r => e("Deposit cash should fail if transfer fails", r),
        }.unwrap();
    }
    {
        balances.insert((btc, alice), u128::MAX);
        setup_call(alice, l_btc, 0, timestamp);
        btc_app.deposit_cash(alice, u128::MAX).unwrap();
    }
    {
        balances.insert((btc, alice), 1);
        setup_call(alice, l_btc, 0, timestamp);
        match btc_app.deposit_cash(alice, 1) {
            Err(LAssetError::DepositCashOverflow) => Ok(()),
            r => e("Deposit cash should fail on overflow", r),
        }.unwrap();
    }
    {
        *transfer_error = true;
        setup_call(alice, l_btc, 0, timestamp);
        match btc_app.withdraw_cash() {
            Err(LAssetError::WithdrawCashTransferFailed(_)) => Ok(()),
            r => e("Withdraw cash should fail if transfer fails", r),
        }.unwrap();
    }
    {
        balances.insert((btc, alice), u128::MAX);
        setup_call(alice, l_btc, 0, timestamp);
        btc_app.deposit_cash(alice, u128::MAX).unwrap();
    }
    {
        balances.insert((btc, alice), 1);
        setup_call(alice, l_btc, 0, timestamp);
        match btc_app.repay(alice, 1) {
            Err(LAssetError::RepayCashOverflow) => Ok(()),
            r => e("Repay cash should fail on overflow", r),
        }.unwrap();
    }
    {
        setup_call(alice, l_btc, 0, timestamp);
        match btc_app.liquidate(bob) {
            Err(LAssetError::LiquidateForNothing) => Ok(()),
            r => e("Liquidate should fail if nothing to liquidate", r),
        }.unwrap();
    }
    {
        setup_call(alice, l_usdc, 0, timestamp);
        match usdc_app.liquidate(alice) {
            Err(LAssetError::LiquidateTooEarly) => Ok(()),
            r => e("Liquidate should fail if too early", r),
        }.unwrap();
    }
    {
        balances.insert((eth, alice), 1);
        setup_call(alice, l_eth, 0, timestamp);
        eth_app.mint(1).unwrap();

        setup_call(alice, l_eth, 1, timestamp);
        eth_app.borrow(1).unwrap();

        *transfer_error = true;
        usdc_app.price_scaler = 2;
        setup_call(alice, l_usdc, 0, timestamp);
        match usdc_app.liquidate(alice) {
            Err(LAssetError::LiquidateTransferFailed(_)) => Ok(()),
            r => e("Liquidate should fail if transfer fails", r),
        }.unwrap();
        usdc_app.price_scaler = 1;
    }
    {
        eth_app.maintenance_margin = u128::MAX;
        btc_app.maintenance_margin = u128::MAX;
        setup_call(alice, l_usdc, 0, timestamp);
        match usdc_app.liquidate(alice) {
            Err(LAssetError::LiquidateTooMuch) => Ok(()),
            r => e("Liquidate should fail if too much", r),
        }.unwrap();
    }
    {
        setup_call(alice, l_btc, 0, timestamp);
        match btc_app.take_cash(0, alice) {
            Err(FlashLoanPoolError::TakeCashUnauthorized) => Ok(()),
            r => e("Take cash should fail if unauthorized", r),
        }.unwrap();
    }
    {
        btc_app.take_cash_fee = 1;
        setup_call(admin, l_btc, 0, timestamp);
        match btc_app.take_cash(u128::MAX, admin) {
            Err(FlashLoanPoolError::TakeCashFeeOverflow) => Ok(()),
            r => e("Take cash should fail on fee overflow", r),
        }.unwrap();
    }
    {
        btc_app.take_cash_fee = u128::MAX;
        setup_call(admin, l_btc, 0, timestamp);
        btc_app.take_cash(u128::MAX / 2, admin).unwrap();
        match btc_app.take_cash(u128::MAX / 2, admin) {
            Err(FlashLoanPoolError::TakeCashOverflow) => Ok(()),
            r => e("Take cash should fail on overflow", r),
        }.unwrap();
    }
    {
        *transfer_error = true;
        btc_app.take_cash_fee = 0;
        setup_call(admin, l_btc, 0, timestamp);
        match btc_app.take_cash(0, admin) {
            Err(FlashLoanPoolError::TakeCashFailed(_)) => Ok(()),
            r => e("Take cash should fail if transfer fails", r),
        }.unwrap();
    }
}