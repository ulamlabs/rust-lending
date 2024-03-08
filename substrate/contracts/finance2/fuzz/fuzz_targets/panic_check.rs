#![no_main]

use libfuzzer_sys::fuzz_target;
use arbitrary::Arbitrary;

#[derive(Arbitrary, Debug)]
enum Method {
    Deposit {
        pubcaller: u8,
        callee: Option<bool>,
        to_deposit: u128,
        value: bool,
    },
    Withdraw {
        time_delta: u32,
        caller: u8,
        callee: Option<bool>,
        to_withdraw: u128,
        transfer_error: bool,
    },
    Mint {
        time_delta: u32,
        caller: u8,
        callee: Option<bool>,
        to_wrap: u128,
        allowance: u128,
    },
    Burn {
        time_delta: u32,
        caller: u8,
        callee: Option<bool>,
        to_burn: u128,
        transfer_error: bool,
    },
    Borrow {
        time_delta: u32,
        caller: u8,
        callee: Option<bool>,
        to_borrow: u128,
        value: bool, 
        transfer_error: bool,
    },
    DepositCash {
        caller: u8,
        callee: Option<bool>,
        extra_cash: u128,
        spender: Result<Option<bool>, u8>,
        allowance: u128,
    },
    WithdrawCash {
        caller: u8,
        callee: Option<bool>,
        transfer_error: bool,
    },
    Liquidate {
        time_delta: u32,
        caller: u8,
        callee: Option<bool>,
        user: u8,
        transfer_error: bool,
    },
    Accrue {
        time_delta: u32,
        caller: u8,
        callee: Option<bool>,
    },
    Repay {
        time_delta: u32,
        caller: u8,
        callee: Option<bool>,
        to_repay: u128,
        user: u8,
        allowance: u128,
    },
    RepayOrUpdate {
        time_delta: u32,
        caller: u8,
        callee: Option<bool>,
        user: u8,
        cash_owner: u8,
    },
    Update {
        time_delta: u32,
        caller: u8,
        callee: Option<bool>,
        user: u8,
    },
    TakeCash {
        admin: bool,
        amount: u128,
        target: u8,
        transfer_error: bool,
    },
    SetPrice {
        admin: bool,
        price: u128,
        price_scaler: u128,
    },
    SetParams {
        admin: bool,
        standard_rate: u128,
        standard_min_rate: u128,
        emergency_rate: u128,
        emergency_max_rate: u128,
        initial_margin: u128,
        maintenance_margin: u128,
        initial_haircut: u128,
        maintenance_haircut: u128,
        mint_fee: u128,
        borrow_fee: u128,
        take_cash_fee: u128,
        liquidation_reward: u128,
    },
    Transfer {
        caller: u8,
        callee: u8,
        amount: u128,
        to: u8,
    },
    TransferFrom {
        caller: u8,
        callee: u8,
        amount: u128,
        from: u8,
        to: u8,
    },
    Approve {
        caller: u8,
        callee: u8,
        amount: u128,
        spender: u8,
    },
    IncreaseAllowance {
        caller: u8,
        callee: u8,
        amount: u128,
        spender: u8,
    },
    DecreaseAllowance {
        caller: u8,
        callee: u8,
        amount: u128,
        spender: u8,
    },
}

fuzz_target!(|method: Method| {  
    // let btc = AccountId::from(BTC_ADDRESS); 
    // let usdc = AccountId::from(USDC_ADDRESS);
    // let eth = AccountId::from(ETH_ADDRESS);  
    // let admin = AccountId::from([0x0; 32]);
    // unsafe {
    //     if BALANCES.is_none() {
    //         BALANCES = Some(std::collections::HashMap::new());
    
    //         setup_call(admin, btc, 0, 0);
    //         L_BTC = Some(LAssetContract::new(btc, usdc, 1));
    //         setup_call(admin, usdc, 0, 0);
    //         L_USDC = Some(LAssetContract::new(usdc, eth, 1));
    //         setup_call(admin, eth, 0, 0);
    //         L_ETH = Some(LAssetContract::new(eth, btc, 1));
    //     }
    // }
    // let (transfer_error, balances, l_btc, l_usdc, l_eth) = unsafe {
    //     (&mut TRANSFER_ERROR, BALANCES.as_mut().unwrap(), L_BTC.as_mut().unwrap(), L_USDC.as_mut().unwrap(), L_ETH.as_mut().unwrap())
    // };
    // for method in methods {
    // }
});
