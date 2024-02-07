#![cfg_attr(not(feature = "std"), no_std, no_main)]

mod errors;

#[ink::contract]
pub mod assetpool {
    use ink::{prelude::vec::Vec, storage_item, contract_ref};
    use psp22::{PSP22Data, PSP22Error, PSP22, PSP22Event};
    use primitive_types::U256;
    use crate::errors::{AssetPoolError, AssetPoolResult};

    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        value: u128,
    }

    #[ink(event)]
    pub struct Approval {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        spender: AccountId,
        amount: u128,
    }

    #[storage_item]
    #[derive(Debug)]
    pub struct PoolData {
        pub manager: AccountId,
        pub token: AccountId,
        pub total_borrowed: u128,
    }

    #[ink(storage)]
    pub struct Pool {
        psp22: PSP22Data,
        data: PoolData,
    }

    impl Pool {
        #[ink(constructor)]
        pub fn new(manager: AccountId, token: AccountId) -> Self {
            let data = PoolData{manager, token, total_borrowed: 0};
            Self {
                psp22: PSP22Data::default(),
                data,
            }
        }

        #[ink(message)]
        pub fn total_deposited(&self) -> u128 {
            psp22_balance_of(self.data.token, self.env().account_id()) + self.data.total_borrowed
        }

        #[ink(message)]
        pub fn deposit(&mut self, amount: u128) -> AssetPoolResult {
            let shares: u128 = if self.psp22.total_supply() == 0 {
                // add total deposited to avoid lAsset inflation attack
                amount + self.total_deposited()
            } else {
                let total_shares = self.psp22.total_supply();
                let total_deposit = psp22_balance_of(self.data.token, self.env().account_id());
                ratio(amount, total_shares, total_deposit)?
            };
            psp22_transfer_from(self.data.token, self.env().caller(), self.env().account_id(), amount)?;
            let evs = self.psp22.mint(self.env().caller(), shares)?;
            self.emit_events(evs);

            // TODO: inform manager about balance change?
            // ...

            // TODO: Add events

            Ok(())
        }

        #[ink(message)]
        pub fn withdraw(&mut self, shares: u128) -> AssetPoolResult {
            let total_shares = self.psp22.total_supply();
            let total_deposited = self.total_deposited();
            let amount = ratio(total_deposited, shares, total_shares)?;
            let evs = self.psp22.burn(self.env().caller(), shares)?;
            self.emit_events(evs);
            psp22_transfer(self.data.token, self.env().caller(), amount)?;

            // TODO: Inform manager about balance change?
            // ...

            // TODO: Add events

            Ok(())
        }

        #[ink(message)]
        pub fn borrow_to(&mut self, amount: u128, to: AccountId) -> AssetPoolResult {
            if self.env().caller() != self.data.manager {
                return Err(AssetPoolError::Unauthorized)
            }
            self.data.total_borrowed += amount;
            psp22_transfer(self.data.token, to, amount)?;

            Ok(())
        }

        #[ink(message)]
        pub fn repay_from(&mut self, amount: u128, from: AccountId) -> AssetPoolResult {
            if self.env().caller() != self.data.manager {
                return Err(AssetPoolError::Unauthorized)
            }
            self.data.total_borrowed -= amount;
            psp22_transfer_from(self.data.token, from, self.env().account_id(), amount)?;

            Ok(())
        }


        fn emit_events(&self, events: Vec<PSP22Event>) {
            for event in events {
                match event {
                    PSP22Event::Transfer { from, to, value } => {
                        self.env().emit_event(Transfer { from, to, value })
                    }
                    PSP22Event::Approval {
                        owner,
                        spender,
                        amount,
                    } => self.env().emit_event(Approval {
                        owner,
                        spender,
                        amount,
                    }),
                }
            }
        }
    }

    impl PSP22 for Pool {
        #[ink(message)]
        fn total_supply(&self) -> u128 {
            self.psp22.total_supply()
        }

        #[ink(message)]
        fn balance_of(&self, owner: AccountId) -> u128 {
            self.psp22.balance_of(owner)
        }

        #[ink(message)]
        fn allowance(&self, owner: AccountId, spender: AccountId) -> u128 {
            self.psp22.allowance(owner, spender)
        }

        #[ink(message)]
        fn transfer(
            &mut self,
            to: AccountId,
            value: u128,
            _data: Vec<u8>,
        ) -> Result<(), PSP22Error> {
            let events = self.psp22.transfer(self.env().caller(), to, value)?;
            self.emit_events(events);
            Ok(())
        }

        #[ink(message)]
        fn transfer_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            value: u128,
            _data: Vec<u8>,
        ) -> Result<(), PSP22Error> {
            let events = self
                .psp22
                .transfer_from(self.env().caller(), from, to, value)?;
            self.emit_events(events);
            Ok(())
        }

        #[ink(message)]
        fn approve(&mut self, spender: AccountId, value: u128) -> Result<(), PSP22Error> {
            let events = self.psp22.approve(self.env().caller(), spender, value)?;
            self.emit_events(events);
            Ok(())
        }

        #[ink(message)]
        fn increase_allowance(
            &mut self,
            spender: AccountId,
            delta_value: u128,
        ) -> Result<(), PSP22Error> {
            let events =
                self.psp22
                    .increase_allowance(self.env().caller(), spender, delta_value)?;
            self.emit_events(events);
            Ok(())
        }

        #[ink(message)]
        fn decrease_allowance(
            &mut self,
            spender: AccountId,
            delta_value: u128,
        ) -> Result<(), PSP22Error> {
            let events =
                self.psp22
                    .decrease_allowance(self.env().caller(), spender, delta_value)?;
            self.emit_events(events);
            Ok(())
        }
    }

    // PSP22 helpers

    #[inline]
    fn psp22_balance_of(token: AccountId, owner: AccountId) -> u128 {
        let token: contract_ref!(PSP22) = token.into();
        token.balance_of(owner)
    }

    #[inline]
    fn psp22_transfer(token: AccountId, to: AccountId, value: u128) -> Result<(), PSP22Error> {
        let mut token: contract_ref!(PSP22) = token.into();
        token.transfer(to, value, Vec::new())
    }

    #[inline]
    fn psp22_transfer_from(
        token: AccountId,
        from: AccountId,
        to: AccountId,
        value: u128,
    ) -> Result<(), PSP22Error> {
        let mut token: contract_ref!(PSP22) = token.into();
        token.transfer_from(from, to, value, Vec::new())
    }

    // Math helpers

    fn ratio(m1: u128, m2: u128, d: u128) -> Result<u128, AssetPoolError>{
        let m1w = U256::from(m1);
        let m2w = U256::from(m2);
        let dw = U256::from(d);

        let res = m1w * dw / m2w;
        res.try_into().map_err(|_| AssetPoolError::RatioCalculationError)
    }
}