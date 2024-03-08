#![no_main]

use libfuzzer_sys::arbitrary::Arbitrary;

#[derive(Arbitrary, Debug)]
enum Method {
    Deposit {
        caller: u8,
        callee: Option<bool>,
        to_deposit: u128,
        value: bool,
    },
    Withdraw {
        caller: u8,
        callee: Option<bool>,
        to_withdraw: u128,
        transfer_error: bool,
    },
    Mint {
        caller: u8,
        callee: Option<bool>,
        to_wrap: u128,
        allowance: u128,
    },
    Burn {
        caller: u8,
        callee: Option<bool>,
        to_burn: u128,
        transfer_error: bool,
    },
    Borrow {
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
        allowance: u128,
    }
}

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // fuzzed code goes here
});
