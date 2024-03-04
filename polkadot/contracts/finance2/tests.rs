use ink::primitives::AccountId;
use traits::psp22::PSP22Error;

use crate::finance2::*;
use crate::errors::LAssetError;

fn setup_call(caller: AccountId, callee: AccountId, value: u128, timestamp: u64) {
    ink::env::test::set_caller::<ink::env::DefaultEnvironment>(caller);
    ink::env::test::set_callee::<ink::env::DefaultEnvironment>(callee);
    ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(timestamp);
    ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(value);
}

fn e<T: std::fmt::Debug>(m: &str, r: Result<T, LAssetError>) -> Result<T, LAssetError> {
    Err(LAssetError::TestError(m.to_string() + format!(". Got: {:?}", r).as_str()))
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
    let (balances, btc_app, usdc_app, eth_app) = unsafe {
        (BALANCES.as_mut().unwrap(), L_BTC.as_mut().unwrap(), L_USDC.as_mut().unwrap(), L_ETH.as_mut().unwrap())
    };

    setup_call(alice, l_btc, 0, timestamp);
    match btc_app.deposit(0) {
        Err(LAssetError::FirstDepositRequiresGasCollateral) => Ok(()),
        r => e("First deposit should fail without gas collateral", r),
    }.unwrap();

    setup_call(alice, l_btc, 1, timestamp);
    btc_app.deposit(0).unwrap();

    setup_call(alice, l_btc, 0, timestamp);
    match btc_app.withdraw(u128::MAX) {
        Err(LAssetError::WithdrawOverflow) => Ok(()),
        r => e("Withdraw should fail on overflow", r),
    }.unwrap();
    
    setup_call(alice, l_btc, 0, timestamp);
    match btc_app.deposit(1) {
        Err(LAssetError::DepositTransferFailed(PSP22Error::InsufficientBalance)) => Ok(()),
        r => e("Deposit should fail if caller has insufficient balance", r),
    }.unwrap();

    balances.insert((btc, alice), u128::MAX);
    setup_call(alice, l_btc, 0, timestamp);
    btc_app.deposit(u128::MAX).unwrap();
    
    setup_call(alice, l_btc, 0, timestamp);
    match btc_app.borrow(0) {
        Err(LAssetError::BorrowWhileDepositingNotAllowed) => Ok(()),
        r => e("Borrow while depositing should fail", r),
    }.unwrap();
    
    balances.insert((btc, alice), 1);
    setup_call(alice, l_btc, 0, timestamp);
    match btc_app.deposit(1) {
        Err(LAssetError::DepositOverflow) => Ok(()),
        r => e("Deposit should fail on overflow", r),
    }.unwrap();

    setup_call(alice, l_btc, 0, timestamp);
    btc_app.withdraw(u128::MAX).unwrap();

    setup_call(bob, l_btc, 0, timestamp);
    match btc_app.withdraw(0) {
        Err(LAssetError::WithdrawWithoutDeposit) => Ok(()),
        r => e("Withdraw without deposit should fail", r),
    }.unwrap();

    setup_call(alice, l_btc, 0, timestamp);
    match btc_app.borrow(0) {
        Err(LAssetError::FirstBorrowRequiresGasCollateral) => Ok(()),
        r => e("First borrow should fail without gas collateral", r),
    }.unwrap();

    setup_call(alice, l_btc, 1, timestamp);
    match btc_app.borrow(u128::MAX) {
        Err(LAssetError::BorrowOverflow) => Ok(()),
        r => e("Borrow should fail on overflow", r),
    }.unwrap();

    setup_call(alice, l_btc, 0, timestamp);
    btc_app.mint(1).unwrap();

    balances.insert((btc, alice), u128::MAX);
    setup_call(alice, l_btc, 0, timestamp);
    match btc_app.mint(u128::MAX) {
        Err(LAssetError::MintOverflow) => Ok(()),
        r => e("Mint liquidity should fail on overflow", r),
    }.unwrap();

    setup_call(alice, l_btc, 0, timestamp);
    match btc_app.burn(u128::MAX) {
        Err(LAssetError::BurnOverflow) => Ok(()),
        r => e("Burn should fail on overflow", r),
    }.unwrap();
}