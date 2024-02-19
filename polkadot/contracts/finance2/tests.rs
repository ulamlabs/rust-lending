use ink::primitives::AccountId;

use super::*;

#[ink::test]
fn default_works() {
    let l_btc = AccountId::from([0x1; 32]);
    let l_usdc = AccountId::from([0x2; 32]);
    let l_eth = AccountId::from([0x3; 32]);
    let admin = AccountId::from([0x4; 32]);
    let btc = AccountId::from([0x5; 32]);
    let usdc = AccountId::from([0x6; 32]);
    let eth = AccountId::from([0x7; 32]);
    unsafe {
        L_BTC = Some(LAssetContract::new(
            admin,
            btc,
            l_usdc,
            l_eth,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        ));
        L_USDC = Some(LAssetContract::new(
            admin,
            usdc,
            l_eth,
            l_btc,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        ));
        L_ETH = Some(LAssetContract::new(
            admin,
            eth,
            l_btc,
            l_usdc,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        ));
    }
    unsafe {
        let btc_app = L_BTC.as_mut().unwrap();
        let usdc_app = L_USDC.as_mut().unwrap();
        let eth_app = L_ETH.as_mut().unwrap();

        run(btc_app, usdc_app, eth_app).unwrap();
    }
}

fn run(btc: &mut LAssetContract, _usdc: &mut LAssetContract, _eth: &mut LAssetContract) -> Result<(), LAssetError> {
    btc.deposit(100)?;
    btc.withdraw(100)?;
    Ok(())
}
