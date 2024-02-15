#![cfg_attr(not(feature = "std"), no_std, no_main)]

use ink::primitives::AccountId;

mod errors;

#[ink::trait_definition]
pub trait LAsset {
    #[ink(message)]
    fn update(&mut self) -> AccountId;
}

#[ink::contract]
mod finance2 {
    use errors::LAssetError;
    #[cfg(not(test))]
    use ink::contract_ref;
    use ink_e2e::subxt::rpc::types::NewBlock;
    use primitive_types::{U128, U256};

    
    //this function assumes c is not zero or a <= c or b <= c
    pub fn ratio(a: u128, b: u128, c: u128) -> u128 {
        let denominator = U128::from(a).full_mul(U128::from(b));
        let result = denominator / U256::from(c);
        result.low_u128()
    }

    use ink::storage::Mapping;
    use crate::{errors::LAssetError, LAsset};

    #[ink(storage)]
    pub struct LAssetContract {
        admin: AccountId,
        asset: AccountId,
        updated_at: Timestamp,

        next: AccountId,
        prev: AccountId,

        total_collateral: u128,
        collaterals: Mapping<AccountId, u128>,
        
        //Maximum amount of liquidity that can be borrowed
        total_liquidity: u128,
        //Sum of all liquidity shares
        total_liquidity_shares: u128,
        //Number of shares owned by each user
        liquidity_shares: Mapping<AccountId, u128>,
        
        //Amount of liquidity that can be borrowed
        //It is better to store it in that way, because
        //It is impossible to forget about check, that someone is borrowing to much
        //It has more optimal, becuase it does not have to be touched during updates
        total_borrowable: u128,
        //Sum of all borrow shares
        total_borrow_shares: u128,
        //Number of shares owned by each user
        borrow_shares: Mapping<AccountId, u128>,

        standard_rate: u128,
        standard_min_rate: u128,

        emergency_rate: u128,
        emergency_max_rate: u128,

        initial_margin: u128,
        maintenance_margin: u128,

        initial_haircut: u128,
        maintenance_haircut: u128,

        price: u128,
        price_scaler: u128,
    }

    #[cfg(test)]
    static mut L_BTC: Option<LAssetContract> = None;
    #[cfg(test)]
    static mut L_USDC: Option<LAssetContract> = None;
    #[cfg(test)]
    static mut L_ETH: Option<LAssetContract> = None;


    impl LAssetContract {
        #[ink(constructor)]
        pub fn new(
            admin: AccountId,
            asset: AccountId,
            next: AccountId,
            prev: AccountId,
            standard_rate: u128,
            standard_min_rate: u128,
            emergency_rate: u128,
            emergency_max_rate: u128,
            initial_margin: u128,
            maintenance_margin: u128,
            initial_haircut: u128,
            maintenance_haircut: u128,
            price: u128,
            price_scaler: u128,
        ) -> Self {
            Self { 
                admin,
                asset,
                updated_at: Self::env().block_timestamp(),
                next,
                prev,
                total_collateral: 0,
                collaterals: Mapping::new(),
                total_liquidity: 0,
                total_liquidity_shares: 0,
                liquidity_shares: Mapping::new(),
                total_borrowable: 0,
                total_borrow_shares: 0,
                borrow_shares: Mapping::new(),
                standard_rate,
                standard_min_rate,
                emergency_rate,
                emergency_max_rate,
                initial_margin,
                maintenance_margin,
                initial_haircut,
                maintenance_haircut,
                price,
                price_scaler,
             }
        }

        #[cfg(not(test))]
        fn update_next(&self) -> AccountId {
            let mut next: contract_ref!(LAsset) = self.next.into();
            next.update()
        }

        #[cfg(test)]
        fn update_next(&self) -> AccountId {
            unsafe {
                if self.next == AccountId::from([0x1; 32]) {
                    return L_BTC.as_mut().unwrap().update();
                }
                if self.next == AccountId::from([0x2; 32]) {
                    return L_USDC.as_mut().unwrap().update();
                }
                if self.next == AccountId::from([0x3; 32]) {
                    return L_ETH.as_mut().unwrap().update();
                }
                unreachable!();
            }
        }

        #[ink(message)]
        pub fn deposit(&mut self, amount: u128) -> Result<(), LAssetError> {
            let env = self.env();
            //You can deposit for yourself only
            let caller = env.caller();

            let old_total_collateral = self.total_collateral;
            //First deposit does not require any extra actions
            let old_collateral = self.collaterals.get(&caller).unwrap_or(0);

            let new_total_collateral = {
                //new_total_collateral is calculated first, because if it doesn't overflow,
                //it is impossible for new_collateral to overflow
                let r = old_total_collateral.checked_add(amount);

                //This check is potential blocker, but it can fail only if
                //token as total supply greater than 2^128
                //We cannot do anything about it, so we just return error
                r.ok_or(LAssetError::DepositOverflow)
            }?;
            //impossible to overflow IF total_collateral is tracked correctly
            let new_collateral = old_collateral + amount;

            //it is crucial to update those two variables together
            self.total_collateral = new_total_collateral;
            self.collaterals.insert(caller, &new_collateral);

            Ok(())
        }

        #[ink(message)]
        pub fn withdraw(&mut self, amount: u128) -> Result<(), LAssetError> {
            let env = self.env();
            //You can withdraw for yourself only
            let caller = env.caller();

            let total_collateral = self.total_collateral;
            //Withdraw without deposit is useless, but not forbidden
            let collateral = self.collaterals.get(&caller).unwrap_or(0);

            let new_collateral = {
                //new_collateral is calculated first, because if it doesn't overflow,
                //it is impossible for new_total_collateral to overflow
                let r = collateral.checked_sub(amount);

                //This check is potential blocker, but it can fail only if
                //caller tries to withdraw more than she has
                r.ok_or(LAssetError::WithdrawOverflow)
            }?;

            //impossible to overflow IF total_collateral is tracked correctly
            let new_total_collateral = total_collateral - amount;

            //it is crucial to update those two variables together
            self.total_collateral = new_total_collateral;
            self.collaterals.insert(caller, &new_collateral);
            Ok(())
        }

        //In this function amount is amount of liquidity, not shares
        //it is hard to predict how much shares will be minted
        #[ink(message)]
        pub fn mint(&mut self, amount: u128) -> Result<(), LAssetError> {
            let env = self.env();
            //You can mint for yourself only
            let caller = env.caller();

            let total_borowable = self.total_borrowable;
            let total_liquidity = self.total_liquidity;
            let total_shares = self.total_liquidity_shares;
            //First mint does not require any extra actions
            let shares = self.liquidity_shares.get(&caller).unwrap_or(0);

            let new_total_liquidity = {
                //new_total_liquidity is calculated first, because if it doesn't overflow,
                //it is impossible for new_shares and new_total_shares to overflow
                let r = total_liquidity.checked_add(amount);

                //This check is potential blocker, but it can fail only if
                //token as total supply greater than 2^128 OR
                //interest is enourmous, then only burn can help
                r.ok_or(LAssetError::MintOverflow)
            }?;
            //Number of minted shares is reduced by division precision
            //It is even possible to mint zero shares, even if some liquidity is added
            //It has good sides, number of shares will never be grater than number of liquidity
            //And it incentives caller to mint more liquidity at once
            //Early minters will get more shares, so it is incentive to hold shares longer
            let minted = if total_liquidity == 0 {
                amount
            } else {
                //total_liquidity is not zero
                //total_shares <= total_liquidity, because
                //liquidity is defined as sum of all shares and interest
                ratio(amount, total_shares, total_liquidity)
                //amount divided by total_liquidity is ratio <= 1
                //if we multiply it by total_shares, we will get number of shares
            };
            //impossible to overflow IF total_liquidity is tracked correctly
            let new_shares = shares + minted;
            let new_total_shares = total_shares + minted;
            let new_total_borrowable = total_borowable + amount;

            //it is crucial to update those four variables together
            self.total_liquidity = new_total_liquidity;
            self.total_liquidity_shares = new_total_shares;
            self.liquidity_shares.insert(caller, &new_shares);
            self.total_borrowable = new_total_borrowable;

            Ok(())
        }

        //in this function amount is amount of shares, not liquidity
        //it is hard to predict how much liquidity will be burned
        //because someone else can mint or burn in the meantime
        #[ink(message)]
        pub fn burn(&mut self, amount: u128) -> Result<(), LAssetError> {
            let env = self.env();
            //You can burn for yourself only
            let caller = env.caller();

            let total_borrowable = self.total_borrowable;
            let total_liquidity = self.total_liquidity;
            let total_shares = self.total_liquidity_shares;
            //Burn without mint is useless, but not forbidden
            let shares = self.liquidity_shares.get(&caller).unwrap_or(0);

            let new_shares = {
                //new_shares is calculated first, because if it doesn't overflow,
                //it is impossible for new_total_shares to overflow
                let r = shares.checked_sub(amount);

                //This check is potential blocker, but it can fail only if
                //caller tries to burn more than she has
                r.ok_or(LAssetError::BurnOverflow)
            }?;

            //Number of withdrawned liquidity is reduced by division precision
            //It is even possible to withdraw zero liquidity, even if some shares are burned
            //It has good sides, number of liquidity will never be grater than number of shares
            //And it incentives caller not to burn shares, but hold them longer
            let withdrawn = if total_shares == 0 {
                0
            } else {
                //total_shares is not zero
                //total_liquidity <= total_shares, because
                //liquidity is defined as sum of all shares plus interest
                ratio(amount, total_liquidity, total_shares)
                //amount divided by total_shares is ratio <= 1
                //if we multiply it by total_liquidity, we will get number of liquidity

                //The case, when new_total_shares is zero, is handled in else branch
                //We don't need to handle it in any special way
                //It would just waste gas, because it would happen only once for popular pools
            };

            //impossible to overflow IF liquidity_shares are tracked correctly
            let new_total_shares = total_shares - amount;
            //impossible to overflow IF total_liquidity is tracked correctly
            let new_total_liquidity = total_liquidity - withdrawn;

            let new_total_borrowable = {
                let r = total_borrowable.checked_sub(withdrawn);

                //This check is potential blocker, but it can fail only if
                //Amount of borrowable goes below zero
                //User should accept the rist, that liquidity used to mint shares
                //Can be borrowed and impossible to withdraw
                //It is expected and if it happend, interest rate should be really high
                //And should soon lead to liquidation
                r.ok_or(LAssetError::BurnTooMuch)
            }?;

            Ok(())
        }
    }

    impl LAsset for LAssetContract {
        #[ink(message)]
        fn update(&mut self) -> AccountId {
            self.next
        }
    }

    #[cfg(test)]
    mod tests {
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
                    1,
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
                    1,
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
                    1,
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
    }
}
