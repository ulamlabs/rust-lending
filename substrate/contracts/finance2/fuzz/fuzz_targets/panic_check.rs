#![no_main]

use finance2::tests::*;

#[derive(arbitrary::Arbitrary, Debug)]
enum Method {
    Deposit {
        caller: u8,
        callee: Option<bool>,
        to_deposit: u128,
        value: bool,
        allowance: u128,
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
        is_admin: bool,
        callee: Option<bool>,
        amount: u128,
        target: u8,
        transfer_error: bool,
    },
    SetPrice {
        is_admin: bool,
        callee: Option<bool>,
        price: u128,
        price_scaler: u128,
    },
    SetParams {
        is_admin: bool,
        callee: Option<bool>,
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
        callee: Option<bool>,
        amount: u128,
        to: u8,
    },
    TransferFrom {
        caller: u8,
        callee: Option<bool>,
        amount: u128,
        from: u8,
        to: u8,
    },
    Approve {
        caller: u8,
        callee: Option<bool>,
        amount: u128,
        spender: u8,
    },
    IncreaseAllowance {
        caller: u8,
        callee: Option<bool>,
        amount: u128,
        spender: u8,
    },
    DecreaseAllowance {
        caller: u8,
        callee: Option<bool>,
        amount: u128,
        spender: u8,
    },
}

pub static mut TIMESTAMP: u64 = 0;

libfuzzer_sys::fuzz_target!(|method: Method| {  
    let btc = AccountId::from(BTC_ADDRESS); 
    let usdc = AccountId::from(USDC_ADDRESS);
    let eth = AccountId::from(ETH_ADDRESS);  
    let admin = AccountId::from([0x0; 32]);
    unsafe {
        if BALANCES.is_none() {
            BALANCES = Some(std::collections::HashMap::new());
    
            setup_call(admin, btc, 0, 0);
            L_BTC = Some(LAssetContract::new(btc, usdc, 1));
            setup_call(admin, usdc, 0, 0);
            L_USDC = Some(LAssetContract::new(usdc, eth, 1));
            setup_call(admin, eth, 0, 0);
            L_ETH = Some(LAssetContract::new(eth, btc, 1));
        }
    }
    let (timestamp, t_error, balances, l_btc, l_usdc, l_eth) = unsafe {
        (&mut TIMESTAMP, &mut TRANSFER_ERROR, BALANCES.as_mut().unwrap(), L_BTC.as_mut().unwrap(), L_USDC.as_mut().unwrap(), L_ETH.as_mut().unwrap())
    };
    match method {
        Method::Deposit { caller, callee, to_deposit, value, allowance } => {
            let caller = AccountId::from([caller; 32]);
            let contract = match callee {
                Some(true) => l_btc,
                Some(false) => l_usdc,
                None => l_eth,
            };
            let callee = match callee {
                Some(true) => btc,
                Some(false) => usdc,
                None => eth,
            };
            let value = value as u128;
            let key = (callee, caller);
            balances.insert(key, allowance);
            setup_call(caller, callee, value, *timestamp);
            let _ = contract.deposit(to_deposit);
            balances.remove(&key);
        },
        Method::Withdraw { time_delta, caller, callee, to_withdraw, transfer_error } => {
            let caller = AccountId::from([caller; 32]);
            let contract = match callee {
                Some(true) => l_btc,
                Some(false) => l_usdc,
                None => l_eth,
            };
            let callee = match callee {
                Some(true) => btc,
                Some(false) => usdc,
                None => eth,
            };
            *timestamp += time_delta as u64;
            setup_call(caller, callee, 0, *timestamp);
            *t_error = transfer_error;
            let _ = contract.withdraw(to_withdraw);
            *t_error = false;
        },
        Method::Mint { time_delta, caller, callee, to_wrap, allowance } => {
            let caller = AccountId::from([caller; 32]);
            let contract = match callee {
                Some(true) => l_btc,
                Some(false) => l_usdc,
                None => l_eth,
            };
            let callee = match callee {
                Some(true) => btc,
                Some(false) => usdc,
                None => eth,
            };
            let key = (callee, caller);
            balances.insert(key, allowance);
            *timestamp += time_delta as u64;
            setup_call(caller, callee, 0, *timestamp);
            let _ = contract.mint(to_wrap);
            balances.remove(&key);
        },
        Method::Burn { time_delta, caller, callee, to_burn, transfer_error } => {
            let caller = AccountId::from([caller; 32]);
            let contract = match callee {
                Some(true) => l_btc,
                Some(false) => l_usdc,
                None => l_eth,
            };
            let callee = match callee {
                Some(true) => btc,
                Some(false) => usdc,
                None => eth,
            };
            *timestamp += time_delta as u64;
            setup_call(caller, callee, 0, *timestamp);
            *t_error = transfer_error;
            let _ = contract.burn(to_burn);
            *t_error = false;
        },
        Method::Borrow { time_delta, caller, callee, to_borrow, value, transfer_error } => {
            let caller = AccountId::from([caller; 32]);
            let contract = match callee {
                Some(true) => l_btc,
                Some(false) => l_usdc,
                None => l_eth,
            };
            let callee = match callee {
                Some(true) => btc,
                Some(false) => usdc,
                None => eth,
            };
            *timestamp += time_delta as u64;
            setup_call(caller, callee, value as u128, *timestamp);
            *t_error = transfer_error;
            let _ = contract.borrow(to_borrow);
            *t_error = false;
        },
        Method::DepositCash { caller, callee, extra_cash, spender, allowance } => {
            let caller = AccountId::from([caller; 32]);
            let contract = match callee {
                Some(true) => l_btc,
                Some(false) => l_usdc,
                None => l_eth,
            };
            let spender = match spender {
                Ok(Some(true)) => btc,
                Ok(Some(false)) => usdc,
                Ok(None) => eth,
                Err(spender) => AccountId::from([spender; 32]),
            };
            let callee = match callee {
                Some(true) => btc,
                Some(false) => usdc,
                None => eth,
            };
            let key = (spender, caller);
            balances.insert(key, allowance);
            setup_call(caller, callee, 0, *timestamp);
            let _ = contract.deposit_cash(spender, extra_cash);
            balances.remove(&key);
        },
        Method::WithdrawCash { caller, callee, transfer_error } => {
            let caller = AccountId::from([caller; 32]);
            let contract = match callee {
                Some(true) => l_btc,
                Some(false) => l_usdc,
                None => l_eth,
            };
            let callee = match callee {
                Some(true) => btc,
                Some(false) => usdc,
                None => eth,
            };
            setup_call(caller, callee, 0, *timestamp);
            *t_error = transfer_error;
            let _ = contract.withdraw_cash();
            *t_error = false;
        },
        Method::Liquidate { time_delta, caller, callee, user, transfer_error } => {
            let caller = AccountId::from([caller; 32]);
            let contract = match callee {
                Some(true) => l_btc,
                Some(false) => l_usdc,
                None => l_eth,
            };
            let user = AccountId::from([user; 32]);
            let callee = match callee {
                Some(true) => btc,
                Some(false) => usdc,
                None => eth,
            };
            *timestamp += time_delta as u64;
            setup_call(caller, callee, 0, *timestamp);
            *t_error = transfer_error;
            let _ = contract.liquidate(user);
            *t_error = false;
        },
        Method::Accrue { time_delta, caller, callee } => {
            let caller = AccountId::from([caller; 32]);
            let contract = match callee {
                Some(true) => l_btc,
                Some(false) => l_usdc,
                None => l_eth,
            };
            let callee = match callee {
                Some(true) => btc,
                Some(false) => usdc,
                None => eth,
            };
            *timestamp += time_delta as u64;
            setup_call(caller, callee, 0, *timestamp);
            let _ = contract.accrue();
        },
        Method::Repay { time_delta, caller, callee, to_repay, user, allowance } => {
            let caller = AccountId::from([caller; 32]);
            let contract = match callee {
                Some(true) => l_btc,
                Some(false) => l_usdc,
                None => l_eth,
            };
            let callee = match callee {
                Some(true) => btc,
                Some(false) => usdc,
                None => eth,
            };
            let user = AccountId::from([user; 32]);
            let key = (callee, caller);
            balances.insert(key, allowance);
            *timestamp += time_delta as u64;
            setup_call(caller, callee, 0, *timestamp);
            let _ = contract.repay(user, to_repay);
            balances.remove(&key);
        },
        Method::RepayOrUpdate { time_delta, caller, callee, user, cash_owner } => {
            let caller = AccountId::from([caller; 32]);
            let contract = match callee {
                Some(true) => l_btc,
                Some(false) => l_usdc,
                None => l_eth,
            };
            let callee = match callee {
                Some(true) => btc,
                Some(false) => usdc,
                None => eth,
            };
            let cash_owner = AccountId::from([cash_owner; 32]);
            let user = AccountId::from([user; 32]);
            *timestamp += time_delta as u64;
            setup_call(caller, callee, 0, *timestamp);
            let _ = contract.repay_or_update(user, cash_owner);
        },
        Method::Update { time_delta, caller, callee, user } => {
            let caller = AccountId::from([caller; 32]);
            let contract = match callee {
                Some(true) => l_btc,
                Some(false) => l_usdc,
                None => l_eth,
            };
            let callee = match callee {
                Some(true) => btc,
                Some(false) => usdc,
                None => eth,
            };
            let user = AccountId::from([user; 32]);
            *timestamp += time_delta as u64;
            setup_call(caller, callee, 0, *timestamp);
            let _ = contract.update(user);
        },
        Method::TakeCash { is_admin, callee, amount, target, transfer_error } => {
            let caller = if is_admin { admin } else { AccountId::from([0x1; 32]) };
            let contract = match callee {
                Some(true) => l_btc,
                Some(false) => l_usdc,
                None => l_eth,
            };
            let callee = match callee {
                Some(true) => btc,
                Some(false) => usdc,
                None => eth,
            };
            let target = AccountId::from([target; 32]);
            setup_call(caller, callee, 0, *timestamp);
            *t_error = transfer_error;
            let _ = contract.take_cash(amount, target);
            *t_error = false;
        },
        Method::SetPrice { is_admin, callee, price, price_scaler } => {
            let caller = if is_admin { admin } else { AccountId::from([0x1; 32]) };
            let contract = match callee {
                Some(true) => l_btc,
                Some(false) => l_usdc,
                None => l_eth,
            };
            let callee = match callee {
                Some(true) => btc,
                Some(false) => usdc,
                None => eth,
            };
            setup_call(caller, callee, 0, *timestamp);
            let _ = contract.set_price(price, price_scaler);
        },
        Method::SetParams { is_admin, callee, standard_rate, standard_min_rate, emergency_rate, emergency_max_rate, initial_margin, maintenance_margin, initial_haircut, maintenance_haircut, mint_fee, borrow_fee, take_cash_fee, liquidation_reward } => {
            let caller = if is_admin { admin } else { AccountId::from([0x1; 32]) };
            let contract = match callee {
                Some(true) => l_btc,
                Some(false) => l_usdc,
                None => l_eth,
            };
            let callee = match callee {
                Some(true) => btc,
                Some(false) => usdc,
                None => eth,
            };
            setup_call(caller, callee, 0, *timestamp);
            let params = AssetParams {
                standard_rate,
                standard_min_rate,
                emergency_rate,
                emergency_max_rate,
                initial_margin,
                maintenance_margin,
                initial_haircut,
                maintenance_haircut,
                mint_fee,
                borrow_fee,
                take_cash_fee,
                liquidation_reward,
            };
            let _ = contract.set_params(params);
        },
        Method::Transfer { caller, callee, amount, to } => {
            let caller = AccountId::from([caller; 32]);
            let contract = match callee {
                Some(true) => l_btc,
                Some(false) => l_usdc,
                None => l_eth,
            };
            let to = AccountId::from([to; 32]);
            let callee = match callee {
                Some(true) => btc,
                Some(false) => usdc,
                None => eth,
            };
            setup_call(caller, callee, 0, *timestamp);
            let _ = contract.transfer(to, amount, vec![]);
        },
        Method::TransferFrom { caller, callee, amount, from, to } => {
            let caller = AccountId::from([caller; 32]);
            let from = AccountId::from([from; 32]);
            let to = AccountId::from([to; 32]);
            let contract = match callee {
                Some(true) => l_btc,
                Some(false) => l_usdc,
                None => l_eth,
            };
            let callee = match callee {
                Some(true) => btc,
                Some(false) => usdc,
                None => eth,
            };
            setup_call(caller, callee, 0, *timestamp);
            let _ = contract.transfer_from(from, to, amount, vec![]);
        },
        Method::Approve { caller, callee, amount, spender } => {
            let caller = AccountId::from([caller; 32]);
            let spender = AccountId::from([spender; 32]);
            let contract = match callee {
                Some(true) => l_btc,
                Some(false) => l_usdc,
                None => l_eth,
            };
            let callee = match callee {
                Some(true) => btc,
                Some(false) => usdc,
                None => eth,
            };
            setup_call(caller, callee, 0, *timestamp);
            let _ = contract.approve(spender, amount);
        },
        Method::IncreaseAllowance { caller, callee, amount, spender } => {
            let caller = AccountId::from([caller; 32]);
            let spender = AccountId::from([spender; 32]);
            let contract = match callee {
                Some(true) => l_btc,
                Some(false) => l_usdc,
                None => l_eth,
            };
            let callee = match callee {
                Some(true) => btc,
                Some(false) => usdc,
                None => eth,
            };
            setup_call(caller, callee, 0, *timestamp);
            let _ = contract.increase_allowance(spender, amount);
        },
        Method::DecreaseAllowance { caller, callee, amount, spender } => {
            let caller = AccountId::from([caller; 32]);
            let spender = AccountId::from([spender; 32]);
            let contract = match callee {
                Some(true) => l_btc,
                Some(false) => l_usdc,
                None => l_eth,
            };
            let callee = match callee {
                Some(true) => btc,
                Some(false) => usdc,
                None => eth,
            };
            setup_call(caller, callee, 0, *timestamp);
            let _ = contract.decrease_allowance(spender, amount);
        },
    }
});
