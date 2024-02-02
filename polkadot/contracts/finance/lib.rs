#![cfg_attr(not(feature = "std"), no_std, no_main)]


#[ink::contract]
pub mod finance {
    use ink::storage::Mapping;
    use primitive_types::{U128, U256};

    #[ink(storage)]
    pub struct Finance {
        admin: AccountId,
        oracle: AccountId,
        balances: Mapping<AccountId, u128>,
        user_balances: Mapping<(AccountId, AccountId), u128>,
        invested: Mapping<AccountId, u128>,
        user_invested: Mapping<(AccountId, AccountId), u128>,
        borrowed: Mapping<AccountId, u128>,
        user_borrowed: Mapping<(AccountId, AccountId), u128>,
        tokens: Mapping<AccountId, bool>,
        prices: Mapping<AccountId, u128>,
        updated_at: u32,
        user_updated_at: Mapping<AccountId, u32>,
        prices_updated_at: Mapping<AccountId, u32>,

        user_total_balance: Mapping<AccountId, u128>,
        user_total_invested: Mapping<AccountId, u128>,
        user_total_borrowed: Mapping<AccountId, u128>,

        user_unpriced_balance: Mapping<AccountId, u128>,
        user_unpriced_invested: Mapping<AccountId, u128>,
        user_unpriced_borrowed: Mapping<AccountId, u128>,

        user_total_balance_value: Mapping<AccountId, u128>,
        user_total_invested_value: Mapping<AccountId, u128>,
        user_total_borrowed_value: Mapping<AccountId, u128>,

        standard_rates: Mapping<AccountId, u128>,
        cumulative_interests: Mapping<AccountId, u128>,
        user_cumulative_interests: Mapping<(AccountId, AccountId), u128>,
    }
    struct UpToDatePrice(u128);

    struct NewUserUpdatedAt(u32);

    struct NewUserTotalBalance(u128);
    struct NewUserTotalInvested(u128);
    struct NewUserTotalBorrowed(u128);

    struct NewUserTotalBalanceValue(u128);
    struct NewUserTotalInvestedValue(u128);
    struct NewUserTotalBorrowedValue(u128);

    struct NewUserUnpricedBalance(u128);
    struct NewUserUnpricedInvested(u128);
    struct NewUserUnpricedBorrowed(u128);

    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    pub enum FinanceError {
        UnpricedBalanceOverflowImpossible,
        UnpricedInvestedOverflowImpossible,
        UnpricedBorrowedOverflowImpossible,
        UserBalanceValueTooHigh,
        UserInvestedValueTooHigh,
        UserBorrowedValueTooHigh,
        UserTotalBalanceValueTooHigh,
        UserTotalInvestedValueTooHigh,
        UserTotalBorrowedValueTooHigh,
        NothingToRedeem,
        NothingToRedeemForUser,
        NothingToRedeemForUserTotal,
        RedeemTooMuch,
        RedeemTooMuchForUser,
        RedeemTooMuchForUserTotal,
        BorrowOverflow,
        UserBorrowOverflow,
        UserBorrowTotalOverflow,
        RedepositTooMuch,
        NothingToRedeposit,
        RedepositTooMuchForUser,
        NothingToRedepositForUser,
        RedepositTooMuchForUserTotal,
        NothingToRedepositForUserTotal,
        InvestOverflow,
        UserInvestOverflow,
        UserInvestTotalOverflow,
        DepositOverflow,
        DepositUserOverflow,
        DepositUserTotalOverflow,
        TokenNotSupported,
        TokenDisabled,
        NothingToWithdraw,
        NothingToWithdrawForUser,
        NothingToWithdrawForUserTotal,
        WithdrawTooMuch,
        WithdrawTooMuchForUser,
        WithdrawTooMuchForUserTotal,
        CallerIsNotAdmin,
        CallerIsNotOracle,
        PriceNotFound,
        PriceNeverUpdatedImpossible,
        PriceOutOfDate,
        PriceNotConfirmedByUser,
        PriceUpdateNotComplete,
        PriceNotUpdatedByUser,
        PriceUpdateForBalanceNotComplete,
        PriceUpdateForInvestedNotComplete,
        PriceUpdateForBorrowedNotComplete,
        UserBalanceDeltaValueOverflow,
        UserBalanceValueOverflow,
        UserCurrentBalanceValueOverflowImpossible,
        UserBalanceReductionOverflowImpossible,
        UserBalanceValueEmptyImpossible,
        UserInvestedDeltaValueOverflow,
        UserInvestedValueOverflow,
        UserCurrentInvestedValueOverflowImpossible,
        UserInvestedValueEmptyImpossible,
        UserInvestedReductionOverflowImpossible,
        UserInvestedOverflowImpossible,
        UserCurrentBorrowedValueOverflowImpossible,
        UserBorrowedDeltaValueOverflow,
        UserBorrowedValueOverflow,
        UserBorrowedReductionOverflowImpossible,
        UserBorrowedValueEmptyImpossible,
        UserBorrowedOverflowImpossible,
        RateDoesNotFitImpossible,
        TimeDeltaOverflowImpossible,
        AccumulatedRateOverflow,
        CalculatedInterestOverflowImpossible,
        #[cfg(any(feature = "std", test, doc))]
        Test(String)
    }

    struct EnabledToken(AccountId);
    struct SupportedToken(AccountId);
    struct WithdrawOnlyToken(AccountId);
    struct RedepositOnlyToken(AccountId);
    struct RedeemOnlyToken(AccountId);

    trait Token {
        fn id(&self) -> AccountId;
    }
    impl Token for EnabledToken {
        fn id(&self) -> AccountId {
            self.0
        }
    }
    impl Token for WithdrawOnlyToken {
        fn id(&self) -> AccountId {
            self.0
        }
    }
    impl Token for RedepositOnlyToken {
        fn id(&self) -> AccountId {
            self.0
        }
    }
    impl Token for RedeemOnlyToken {
        fn id(&self) -> AccountId {
            self.0
        }
    }
    impl Token for SupportedToken {
        fn id(&self) -> AccountId {
            self.0
        }
    }

    trait DeprecatedToken: Token {}

    
    impl DeprecatedToken for RedepositOnlyToken {}
    impl DeprecatedToken for WithdrawOnlyToken {}
    impl DeprecatedToken for RedeemOnlyToken {}
    trait ActiveToken: Token {}
    impl ActiveToken for EnabledToken {}
    impl ActiveToken for RedepositOnlyToken {}

    struct NewTokenBalance(u128);
    struct NewUserBalance(u128, u128);

    struct NewTokenBorrowed(u128);
    struct NewUserBorrowed(u128, u128);

    struct NewUserInvested(u128, u128);
    struct NewTokenInvested(u128);
    struct User(AccountId);
    struct AdminCaller();
    struct OracleCaller();
    struct Block(u32);
    struct NewUpdatedAt(u32);
    struct NewPriceUpdatedAt(u32, u128, u32);

    struct Rate(u128);
    struct Interest(u128);

    impl Finance {
        /// Creates a new flipper smart contract initialized with the given value.
        #[ink(constructor)]
        pub fn new(oracle: AccountId) -> Self {
            let admin = Self::env().caller();
            let updated_at: u32 = Self::env().block_number();
            Finance {
                admin,
                oracle,
                balances: Mapping::default(),
                user_balances: Mapping::default(),
                invested: Mapping::default(),
                user_invested: Mapping::default(),
                borrowed: Mapping::default(),
                user_borrowed: Mapping::default(),
                tokens: Mapping::default(),
                prices: Mapping::default(),
                updated_at,
                user_updated_at: Mapping::default(),
                prices_updated_at: Mapping::default(),
                user_total_balance: Mapping::default(),
                user_total_invested: Mapping::default(),
                user_total_borrowed: Mapping::default(),
        
                user_unpriced_balance: Mapping::default(),
                user_unpriced_invested: Mapping::default(),
                user_unpriced_borrowed: Mapping::default(),
        
                user_total_balance_value: Mapping::default(),
                user_total_invested_value: Mapping::default(),
                user_total_borrowed_value: Mapping::default(),

                standard_rates: Mapping::default(),
                cumulative_interests: Mapping::default(),
                user_cumulative_interests: Mapping::default(),
            }
        }

        fn forwarded_user(&self, user: AccountId, _: &OracleCaller) -> User {
            User(user)
        }

        fn enabled_token(&self, token: AccountId) -> Result<EnabledToken, FinanceError> {
            if let Some(v) = self.tokens.get(token) {
                if v {
                    Ok(EnabledToken(token))
                } else {
                    Err(FinanceError::TokenDisabled)
                }
            } else {
                Err(FinanceError::TokenNotSupported)
            }
        }

        fn supported_token(&self, token: AccountId) -> Result<SupportedToken, FinanceError> {
            if let None = self.tokens.get(token) {
                Err(FinanceError::TokenNotSupported)
            } else {
                Ok(SupportedToken(token))
            }
        }

        fn redeposit_only_token(&self, token: AccountId) -> RedepositOnlyToken {
            RedepositOnlyToken(token)
        }

        fn redeem_only_token(&self, token: AccountId) -> RedeemOnlyToken {
            RedeemOnlyToken(token)
        }

        fn new_token_balance_after_deposit(&self, token: &impl ActiveToken, amount: u128) -> Result<NewTokenBalance, FinanceError> {
            if let Some(balance) = self.balances.get(token.id()) {
                if let Some(new_balance) = balance.checked_add(amount) {
                    Ok(NewTokenBalance(new_balance))
                } else {
                    Err(FinanceError::DepositOverflow)
                }
            } else {
                Ok(NewTokenBalance(amount))
            }
        }

        fn get_user_balance(&self, token: &impl Token, user: &User) -> Option<u128> {
            self.user_balances.get((token.id(), user.0))
        }

        fn get_user_invested(&self, token: &impl Token, user: &User) -> Option<u128> {
            self.user_invested.get((token.id(), user.0))
        }

        fn set_user_balance(&mut self, token: &impl Token, user: &User, new_user_balance: NewUserBalance) {
            self.user_balances.insert((token.id(), user.0), &new_user_balance.0);
        }

        fn set_token_balance(&mut self, token: &impl Token, new_balance: NewTokenBalance) {
            self.balances.insert(token.id(), &new_balance.0);
        }

        fn set_user_invested(&mut self, token: &impl Token, user: &User, new_user_invested: NewUserInvested) {
            self.user_invested.insert((token.id(), user.0), &new_user_invested.0);
        }

        fn set_token_invested(&mut self, token: &impl Token, new_token_invested: NewTokenInvested) {
            self.invested.insert(token.id(), &new_token_invested.0);
        }

        fn get_user_borrowed(&self, token: &impl Token, user: &User) -> Option<u128> {
            self.user_borrowed.get((token.id(), user.0))
        }

        fn set_token_borrowed(&mut self, token: &impl Token, new_borrowed: NewTokenBorrowed) {
            self.borrowed.insert(token.id(), &new_borrowed.0);
        }

        fn set_user_borrowed(&mut self, token: &impl Token, user: &User, new_user_borrowed: NewUserBorrowed) {
            self.user_borrowed.insert((token.id(), user.0), &new_user_borrowed.0);
        }

        fn set_user_total_balance(&mut self, user: &User, new_user_total_balance: NewUserTotalBalance) {
            self.user_total_balance.insert(user.0, &new_user_total_balance.0);
        }

        fn set_user_total_invested(&mut self, user: &User, new_user_total_invested: NewUserTotalInvested) {
            self.user_total_invested.insert(user.0, &new_user_total_invested.0);
        }

        fn set_user_total_borrowed(&mut self, user: &User, new_user_total_borrowed: NewUserTotalBorrowed) {
            self.user_total_borrowed.insert(user.0, &new_user_total_borrowed.0);
        }


        fn new_user_balance_after_deposit(&self, token: &impl ActiveToken, user: &User, amount: u128) -> Result<NewUserBalance, FinanceError> {
            if let Some(user_balance) = self.get_user_balance(token, user) {
                if let Some(new_user_balance) = user_balance.checked_add(amount) {
                    Ok(NewUserBalance(new_user_balance, user_balance))
                } else {
                    Err(FinanceError::DepositUserOverflow)
                }
            } else {
                Ok(NewUserBalance(amount, 0))
            }
        }

        fn new_user_total_balance_after_deposit(&self, user: &User, amount: u128) -> Result<NewUserTotalBalance, FinanceError> {
            if let Some(user_total_balance) = self.user_total_balance.get(user.0) {
                if let Some(new_user_total_balance) = user_total_balance.checked_add(amount) {
                    Ok(NewUserTotalBalance(new_user_total_balance))
                } else {
                    Err(FinanceError::DepositUserTotalOverflow)
                }
            } else {
                Ok(NewUserTotalBalance(amount))
            }
        }

        fn caller(&self) -> User {
            let e = self.env();
            User(e.caller())
        }

        fn block_number(&self) -> Block {
            Block(self.env().block_number())
        }


        fn oracle_caller(&self) -> Result<OracleCaller, FinanceError> {
            let user = self.caller();
            if user.0 == self.oracle {
                Ok(OracleCaller())
            } else {
                Err(FinanceError::CallerIsNotOracle)
            }
        }

        fn admin_caller(&self) -> Result<AdminCaller, FinanceError> {
            let user = self.caller();
            if user.0 == self.admin {
                Ok(AdminCaller())
            } else {
                Err(FinanceError::CallerIsNotAdmin)
            }
        }

        fn set_token(&mut self, token: &AccountId, _: &AdminCaller, v: bool) {
            self.tokens.insert(token, &v);
        }

        fn set_updated_at(&mut self, new_updated_at: &Option<NewUpdatedAt>, _: &OracleCaller) {
            if let Some(updated_at) = new_updated_at {
                self.updated_at = updated_at.0;
            }
        }

        fn set_price_updated_at(&mut self, token: &SupportedToken, new_price_updated_at: &Option<NewPriceUpdatedAt>, _: &OracleCaller) {
            if let Some(price_updated_at) = new_price_updated_at {
                self.prices_updated_at.insert(token.0, &price_updated_at.0);
                self.prices.insert(token.0, &price_updated_at.1);
            }
        }

        fn set_user_updated_at(&mut self, user: &User, new_user_updated_at: &Option<NewUserUpdatedAt>, _: &OracleCaller) {
            if let Some(user_updated_at) = new_user_updated_at {
                self.user_updated_at.insert(user.0, &user_updated_at.0);
            }
        }

        fn set_user_unpriced_balance(&mut self, user: &User, new_user_unpriced_balance: NewUserUnpricedBalance) {
            self.user_unpriced_balance.insert(user.0, &new_user_unpriced_balance.0);
        }

        fn set_user_unpriced_invested(&mut self, user: &User, new_user_unpriced_invested: NewUserUnpricedInvested) {
            self.user_unpriced_invested.insert(user.0, &new_user_unpriced_invested.0);
        }

        fn set_user_unpriced_borrowed(&mut self, user: &User, new_user_unpriced_borrowed: NewUserUnpricedBorrowed) {
            self.user_unpriced_borrowed.insert(user.0, &new_user_unpriced_borrowed.0);
        }

        fn set_user_total_balance_value(&mut self, user: &User, new_user_total_balance_value: NewUserTotalBalanceValue) {
            self.user_total_balance_value.insert(user.0, &new_user_total_balance_value.0);
        }

        fn set_user_total_invested_value(&mut self, user: &User, new_user_total_invested_value: NewUserTotalInvestedValue) {
            self.user_total_invested_value.insert(user.0, &new_user_total_invested_value.0);
        }

        fn set_user_total_borrowed_value(&mut self, user: &User, new_user_total_borrowed_value: NewUserTotalBorrowedValue) {
            self.user_total_borrowed_value.insert(user.0, &new_user_total_borrowed_value.0);
        }

        fn withdraw_only_token(&self, token: AccountId) -> WithdrawOnlyToken {
            WithdrawOnlyToken(token)
        }

        fn new_token_balance_after_withdraw(&self, token: &impl Token, amount: u128) -> Result<NewTokenBalance, FinanceError> {
            if let Some(balance) = self.balances.get(token.id()) {
                if let Some(new_balance) = balance.checked_sub(amount) {
                    Ok(NewTokenBalance(new_balance))
                } else {
                    Err(FinanceError::WithdrawTooMuch)
                }
            } else {
                Err(FinanceError::NothingToWithdraw)
            }
        }

        fn new_user_balance_after_withdraw(&self, token: &impl Token, user: &User, amount: u128) -> Result<NewUserBalance, FinanceError> {
            if let Some(user_balance) = self.get_user_balance(token, user) {
                if let Some(new_user_balance) = user_balance.checked_sub(amount) {
                    Ok(NewUserBalance(new_user_balance, user_balance))
                } else {
                    Err(FinanceError::WithdrawTooMuchForUser)
                }
            } else {
                Err(FinanceError::NothingToWithdrawForUser)
            }
        }

        fn new_user_total_balance_after_withdraw(&self, user: &User, amount: u128) -> Result<NewUserTotalBalance, FinanceError> {
            if let Some(user_total_balance) = self.user_total_balance.get(user.0) {
                if let Some(new_user_total_balance) = user_total_balance.checked_sub(amount) {
                    Ok(NewUserTotalBalance(new_user_total_balance))
                } else {
                    Err(FinanceError::WithdrawTooMuchForUserTotal)
                }
            } else {
                Err(FinanceError::NothingToWithdrawForUserTotal)
            }
        }


        fn new_user_invested_after_invest(&self, token: &EnabledToken, user: &User, amount: u128) -> Result<NewUserInvested, FinanceError> {
            if let Some(user_invested) = self.get_user_invested(token, user) {
                if let Some(new_user_invested) = user_invested.checked_add(amount) {
                    Ok(NewUserInvested(new_user_invested, user_invested))
                } else {
                    Err(FinanceError::UserInvestOverflow)
                }
            } else {
                Ok(NewUserInvested(amount,0 ))
            }
        }

        fn new_user_total_invested_after_invest(&self, user: &User, amount: u128) -> Result<NewUserTotalInvested, FinanceError> {
            if let Some(user_total_invested) = self.user_total_invested.get(user.0) {
                if let Some(new_user_total_invested) = user_total_invested.checked_add(amount) {
                    Ok(NewUserTotalInvested(new_user_total_invested))
                } else {
                    Err(FinanceError::UserInvestTotalOverflow)
                }
            } else {
                Ok(NewUserTotalInvested(amount))
            }
        }

        fn new_token_invested_after_invest(&self, token: &EnabledToken, amount: u128) -> Result<NewTokenInvested, FinanceError> {
            if let Some(invested) = self.invested.get(token.id()) {
                if let Some(new_invested) = invested.checked_add(amount) {
                    Ok(NewTokenInvested(new_invested))
                } else {
                    Err(FinanceError::InvestOverflow)
                }
            } else {
                Ok(NewTokenInvested(amount))
            }
        }

        fn new_token_invested_after_redeposit(&self, token: &impl DeprecatedToken, amount: u128) -> Result<NewTokenInvested, FinanceError> {
            if let Some(invested) = self.invested.get(token.id()) {
                if let Some(new_invested) = invested.checked_sub(amount) {
                    Ok(NewTokenInvested(new_invested))
                } else {
                    Err(FinanceError::RedepositTooMuch)
                }
            } else {
                Err(FinanceError::NothingToRedeposit)
            }
        }

        fn new_user_invested_after_redeposit(&self, token: &impl DeprecatedToken, user: &User, amount: u128) -> Result<NewUserInvested, FinanceError> {
            if let Some(user_invested) = self.get_user_invested(token, user) {
                if let Some(new_user_invested) = user_invested.checked_sub(amount) {
                    Ok(NewUserInvested(new_user_invested, user_invested))
                } else {
                    Err(FinanceError::RedepositTooMuchForUser)
                }
            } else {
                Err(FinanceError::NothingToRedepositForUser)
            }
        }

        fn new_user_total_invested_after_redeposit(&self, user: &User, amount: u128) -> Result<NewUserTotalInvested, FinanceError> {
            if let Some(user_total_invested) = self.user_total_invested.get(user.0) {
                if let Some(new_user_total_invested) = user_total_invested.checked_sub(amount) {
                    Ok(NewUserTotalInvested(new_user_total_invested))
                } else {
                    Err(FinanceError::RedepositTooMuchForUserTotal)
                }
            } else {
                Err(FinanceError::NothingToRedepositForUserTotal)
            }
        }

        fn new_token_borrowed_after_borrow(&self, token: &impl ActiveToken, amount: u128) -> Result<NewTokenBorrowed, FinanceError> {
            if let Some(borrowed) = self.borrowed.get(token.id()) {
                if let Some(new_borrowed) = borrowed.checked_add(amount) {
                    Ok(NewTokenBorrowed(new_borrowed))
                } else {
                    Err(FinanceError::BorrowOverflow)
                }
            } else {
                Ok(NewTokenBorrowed(amount))
            }
        }

        fn new_user_borrowed_after_borrow(&self, token: &impl ActiveToken, user: &User, amount: u128) -> Result<NewUserBorrowed, FinanceError> {
            if let Some(user_borrowed) = self.get_user_borrowed(token, user) {
                if let Some(new_user_borrowed) = user_borrowed.checked_add(amount) {
                    Ok(NewUserBorrowed(new_user_borrowed, user_borrowed))
                } else {
                    Err(FinanceError::UserBorrowOverflow)
                }
            } else {
                Ok(NewUserBorrowed(amount, 0))
            }
        }

        fn new_user_total_borrowed_after_borrow(&self, user: &User, amount: u128) -> Result<NewUserTotalBorrowed, FinanceError> {
            if let Some(user_total_borrowed) = self.user_total_borrowed.get(user.0) {
                if let Some(new_user_total_borrowed) = user_total_borrowed.checked_add(amount) {
                    Ok(NewUserTotalBorrowed(new_user_total_borrowed))
                } else {
                    Err(FinanceError::UserBorrowTotalOverflow)
                }
            } else {
                Ok(NewUserTotalBorrowed(amount))
            }
        }

        fn new_user_total_borrowed_after_redeem(&self, user: &User, amount: u128) -> Result<NewUserTotalBorrowed, FinanceError> {
            if let Some(user_total_borrowed) = self.user_total_borrowed.get(user.0) {
                if let Some(new_user_total_borrowed) = user_total_borrowed.checked_sub(amount) {
                    Ok(NewUserTotalBorrowed(new_user_total_borrowed))
                } else {
                    Err(FinanceError::RedeemTooMuchForUserTotal)
                }
            } else {
                Err(FinanceError::NothingToRedeemForUserTotal)
            }
        }

        fn new_user_borrowed_after_redeem(&self, token: &impl DeprecatedToken, user: &User, amount: u128) -> Result<NewUserBorrowed, FinanceError> {
            if let Some(user_borrowed) = self.get_user_borrowed(token, user) {
                if let Some(new_user_borrowed) = user_borrowed.checked_sub(amount) {
                    Ok(NewUserBorrowed(new_user_borrowed, user_borrowed))
                } else {
                    Err(FinanceError::RedeemTooMuchForUser)
                }
            } else {
                Err(FinanceError::NothingToRedeemForUser)
            }
        }

        fn new_token_borrowed_after_redeem(&self, token: &impl DeprecatedToken, amount: u128) -> Result<NewTokenBorrowed, FinanceError> {
            if let Some(borrowed) = self.borrowed.get(token.id()) {
                if let Some(new_borrowed) = borrowed.checked_sub(amount) {
                    Ok(NewTokenBorrowed(new_borrowed))
                } else {
                    Err(FinanceError::RedeemTooMuch)
                }
            } else {
                Err(FinanceError::NothingToRedeem)
            }
        }

        fn new_updated_at(&self, block: &Block) -> Option<NewUpdatedAt> {
            if self.updated_at == block.0 {
                None
            } else {
                Some(NewUpdatedAt(block.0))
            }
        }

        fn new_price_updated_at(&self, token: &SupportedToken, block: &Block, price: u128, new_updated_at: &Option<NewUpdatedAt>) -> Option<NewPriceUpdatedAt> {
            let price_updated_at = self.prices_updated_at.get(token.0);
            let is_new = if let Some(price_updated_at) = price_updated_at {
                if let None = new_updated_at {
                    if price_updated_at == self.updated_at {
                        false
                    } else {
                        true
                    }
                } else {
                    true
                }
            } else {
                true
            };
            if is_new {
                let old_price_updated_at = if let Some(price_updated_at) = price_updated_at {
                    price_updated_at
                } else {
                    block.0
                };
                Some(NewPriceUpdatedAt(block.0, price, old_price_updated_at))
            } else {
                None
            }
        }

        fn new_user_updated_at(&self, user: &User, block: &Block, new_updated_at: &Option<NewUpdatedAt>) -> Option<NewUserUpdatedAt> {
            let is_new = if let Some(user_updated_at) = self.user_updated_at.get(user.0) {
                if let None = new_updated_at {
                    if user_updated_at == self.updated_at {
                        false
                    } else {
                        true
                    }
                } else {
                    true
                }
            } else {
                true
            };
            if is_new {
                Some(NewUserUpdatedAt(block.0))
            } else {
                None
            }
        }

        fn new_user_unpriced_balance(&self, token: &SupportedToken, user: &User, new_user_updated_at: &Option<NewUserUpdatedAt>) -> Result<NewUserUnpricedBalance, FinanceError> {
            let base_unpriced_balance = if let Some(_) = new_user_updated_at {
                if let Some(user_total_balance) = self.user_total_balance.get(user.0) {
                    user_total_balance
                } else {
                    0
                }
            } else {
                if let Some(user_unpriced_balance) = self.user_unpriced_balance.get(user.0) {
                    user_unpriced_balance
                } else {
                    0
                }
            };
            if let Some(user_token_balance) = self.get_user_balance(token, user) {
                if let Some(new_unpriced_balance) = base_unpriced_balance.checked_sub(user_token_balance) {
                    Ok(NewUserUnpricedBalance(new_unpriced_balance))
                } else {
                    Err(FinanceError::UnpricedBalanceOverflowImpossible)
                }
            } else {
                Ok(NewUserUnpricedBalance(base_unpriced_balance))
            }
        }

        fn new_user_unpriced_invested(&self, token: &SupportedToken, user: &User, new_user_updated_at: &Option<NewUserUpdatedAt>) -> Result<NewUserUnpricedInvested, FinanceError> {
            let base_unpriced_invested = if let Some(_) = new_user_updated_at {
                if let Some(user_total_invested) = self.user_total_invested.get(user.0) {
                    user_total_invested
                } else {
                    0
                }
            } else {
                if let Some(user_unpriced_invested) = self.user_unpriced_invested.get(user.0) {
                    user_unpriced_invested
                } else {
                    0
                }
            };
            if let Some(user_token_invested) = self.get_user_invested(token, user) {
                if let Some(new_unpriced_invested) = base_unpriced_invested.checked_sub(user_token_invested) {
                    Ok(NewUserUnpricedInvested(new_unpriced_invested))
                } else {
                    Err(FinanceError::UnpricedInvestedOverflowImpossible)
                }
            } else {
                Ok(NewUserUnpricedInvested(base_unpriced_invested))
            }
        }
        fn new_user_unpriced_borrowed(&self, token: &SupportedToken, user: &User, new_user_updated_at: &Option<NewUserUpdatedAt>) -> Result<NewUserUnpricedBorrowed, FinanceError> {
            let base_unpriced_borrowed = if let None = new_user_updated_at {
                if let Some(user_unpriced_borrowed) = self.user_unpriced_borrowed.get(user.0) {
                    user_unpriced_borrowed
                } else {
                    0
                }
            } else {
                if let Some(user_total_borrowed) = self.user_total_borrowed.get(user.0) {
                    user_total_borrowed
                } else {
                    0
                }
            };
            if let Some(user_token_borrowed) = self.get_user_borrowed(token, user) {
                if let Some(new_unpriced_borrowed) = base_unpriced_borrowed.checked_sub(user_token_borrowed) {
                    Ok(NewUserUnpricedBorrowed(new_unpriced_borrowed))
                } else {
                    Err(FinanceError::UnpricedBorrowedOverflowImpossible)
                }
            } else {
                Ok(NewUserUnpricedBorrowed(base_unpriced_borrowed))
            }
        }

        fn new_user_total_balance_value(&self, token: &SupportedToken, user: &User, price: u128, new_user_updated_at: &Option<NewUserUpdatedAt>) -> Result<NewUserTotalBalanceValue, FinanceError> {
            let user_base_balance_value = if let None = new_user_updated_at {
                if let Some(user_total_balance_value) = self.user_total_balance_value.get(user.0) {
                    user_total_balance_value
                } else {
                    0
                }
            } else {
                0
            };
            let user_token_balance_value = if let Some(token_balance) = self.get_user_balance(token, user) {
                if let Some(token_balance_value) = token_balance.checked_mul(price) {
                    Ok(token_balance_value)
                } else {
                    Err(FinanceError::UserBalanceValueTooHigh)
                }
            } else {
                Ok(0)
            }?;
            if let Some(new_total_balance_value) = user_base_balance_value.checked_add(user_token_balance_value) {
                Ok(NewUserTotalBalanceValue(new_total_balance_value))
            } else {
                Err(FinanceError::UserTotalBalanceValueTooHigh)
            }
        }

        fn new_user_total_invested_value(&self, token: &SupportedToken, user: &User, price: u128, new_user_updated_at: &Option<NewUserUpdatedAt>) -> Result<NewUserTotalInvestedValue, FinanceError> {
            let user_base_invested_value = if let None = new_user_updated_at {
                if let Some(user_total_invested_value) = self.user_total_invested_value.get(user.0) {
                    user_total_invested_value
                } else {
                    0
                }
            } else {
                0
            };
            let user_token_invested_value = if let Some(token_invested) = self.get_user_invested(token, user) {
                if let Some(token_invested_value) = token_invested.checked_mul(price) {
                    Ok(token_invested_value)
                } else {
                    Err(FinanceError::UserInvestedValueTooHigh)
                }
            } else {
                Ok(0)
            }?;
            if let Some(new_total_invested_value) = user_base_invested_value.checked_add(user_token_invested_value) {
                Ok(NewUserTotalInvestedValue(new_total_invested_value))
            } else {
                Err(FinanceError::UserTotalInvestedValueTooHigh)
            }
        }

        fn new_user_total_borrowed_value(&self, token: &SupportedToken, user: &User, price: u128, new_user_updated_at: &Option<NewUserUpdatedAt>) -> Result<NewUserTotalBorrowedValue, FinanceError> {
            let user_base_borrowed_value = if let None = new_user_updated_at {
                if let Some(user_total_borrowed_value) = self.user_total_borrowed_value.get(user.0) {
                    user_total_borrowed_value
                } else {
                    0
                }
            } else {
                0
            };
            let user_token_borrowed_value = if let Some(user_borrowed) = self.get_user_borrowed(token, user) {
                if let Some(token_borrowed_value) = user_borrowed.checked_mul(price) {
                    Ok(token_borrowed_value)
                } else {
                    Err(FinanceError::UserBorrowedValueTooHigh)
                }
            } else {
                Ok(0)
            }?;
            if let Some(new_total_borrowed_value) = user_base_borrowed_value.checked_add(user_token_borrowed_value) {
                Ok(NewUserTotalBorrowedValue(new_total_borrowed_value))
            } else {
                Err(FinanceError::UserTotalBorrowedValueTooHigh)
            }
        }

        fn up_to_date_price(&self, token: &impl Token, user: &User) -> Result<UpToDatePrice, FinanceError> {
            let price = if let Some(price) = self.prices.get(token.id()) {
                Ok(price)
            } else {
                Err(FinanceError::PriceNotFound)
            }?;
            let price_updated_at = if let Some(price_updated_at) = self.prices_updated_at.get(token.id()) {
                Ok(price_updated_at)
            } else {
                Err(FinanceError::PriceNeverUpdatedImpossible)
            }?;
            let user_updated_at = if let Some(user_updated_at) = self.user_updated_at.get(user.0) {
                Ok(user_updated_at)
            } else {
                Err(FinanceError::PriceNotUpdatedByUser)
            }?;
            if price_updated_at != self.updated_at {
                return Err(FinanceError::PriceOutOfDate);
            }
            if user_updated_at != self.updated_at {
                return Err(FinanceError::PriceNotConfirmedByUser);
            }
            if let Some(unpriced_balance) = self.user_unpriced_balance.get(user.0) {
                if unpriced_balance != 0 {
                    return Err(FinanceError::PriceUpdateForBalanceNotComplete);
                }
            }
            if let Some(unpriced_invested) = self.user_unpriced_invested.get(user.0) {
                if unpriced_invested != 0 {
                    return Err(FinanceError::PriceUpdateForInvestedNotComplete);
                }
            }
            if let Some(unpriced_borrowed) = self.user_unpriced_borrowed.get(user.0) {
                if unpriced_borrowed != 0 {
                    return Err(FinanceError::PriceUpdateForBorrowedNotComplete);
                }
            }
            Ok(UpToDatePrice(price))
        }

        fn updated_user_total_balance_value(&self, user: &User, price: &UpToDatePrice, new_user_balance: &NewUserBalance) -> Result<NewUserTotalBalanceValue, FinanceError> {
            let user_total_balance_value = if let Some(user_total_balance_value) = self.user_total_balance_value.get(user.0) {
                Ok(user_total_balance_value)
            } else {
                Err(FinanceError::UserBalanceValueEmptyImpossible)
            }?;
            let old_user_balance_value = if let Some(old_user_balance_value) = new_user_balance.1.checked_mul(price.0) {
                Ok(old_user_balance_value)
            } else {
                Err(FinanceError::UserCurrentBalanceValueOverflowImpossible)
            }?;
            let base_user_balance_value = if let Some(base_user_balance_value) = user_total_balance_value.checked_sub(old_user_balance_value) {
                Ok(base_user_balance_value)
            } else {
                Err(FinanceError::UserBalanceReductionOverflowImpossible)
            }?;
            let new_user_balance_value = if let Some(new_user_balance_value) = new_user_balance.0.checked_mul(price.0) {
                Ok(new_user_balance_value)
            } else {
                Err(FinanceError::UserBalanceValueOverflow)
            }?;
            if let Some(new_total_user_balance_value) = base_user_balance_value.checked_add(new_user_balance_value) {
                Ok(NewUserTotalBalanceValue(new_total_user_balance_value))
            } else {
                Err(FinanceError::UserTotalBalanceValueTooHigh)
            }
        }

        fn updated_user_total_invested(&self, user: &User, price: &UpToDatePrice, new_user_invested: &NewUserInvested) -> Result<NewUserTotalInvestedValue, FinanceError> {
            let user_total_invested_value = if let Some(user_total_invested_value) = self.user_total_invested_value.get(user.0) {
                Ok(user_total_invested_value)
            } else {
                Err(FinanceError::UserInvestedValueEmptyImpossible)
            }?;
            let old_user_invested_value = if let Some(old_user_invested_value) = new_user_invested.1.checked_mul(price.0) {
                Ok(old_user_invested_value)
            } else {
                Err(FinanceError::UserCurrentInvestedValueOverflowImpossible)
            }?;
            let base_user_invested_value = if let Some(base_user_invested_value) = user_total_invested_value.checked_sub(old_user_invested_value) {
                Ok(base_user_invested_value)
            } else {
                Err(FinanceError::UserInvestedReductionOverflowImpossible)
            }?;
            let new_user_invested_value = if let Some(new_user_invested_value) = new_user_invested.0.checked_mul(price.0) {
                Ok(new_user_invested_value)
            } else {
                Err(FinanceError::UserInvestedValueOverflow)
            }?;
            if let Some(new_total_user_invested_value) = base_user_invested_value.checked_add(new_user_invested_value) {
                Ok(NewUserTotalInvestedValue(new_total_user_invested_value))
            } else {
                Err(FinanceError::UserTotalInvestedValueTooHigh)
            }
        }

        fn updated_user_total_borrowed(&self, user: &User, price: &UpToDatePrice, new_user_borrowed: &NewUserBorrowed) -> Result<NewUserTotalBorrowedValue, FinanceError> {
            let user_total_borrowed_value = if let Some(user_total_borrowed_value) = self.user_total_borrowed_value.get(user.0) {
                Ok(user_total_borrowed_value)
            } else {
                Err(FinanceError::UserBorrowedValueEmptyImpossible)
            }?;
            let old_user_borrowed_value = if let Some(old_user_borrowed_value) = new_user_borrowed.1.checked_mul(price.0) {
                Ok(old_user_borrowed_value)
            } else {
                Err(FinanceError::UserCurrentBorrowedValueOverflowImpossible)
            }?;
            let base_user_borrowed_value = if let Some(base_user_borrowed_value) = user_total_borrowed_value.checked_sub(old_user_borrowed_value) {
                Ok(base_user_borrowed_value)
            } else {
                Err(FinanceError::UserBorrowedDeltaValueOverflow)
            }?;
            let new_user_borrowed_value = if let Some(new_user_borrowed_value) = new_user_borrowed.0.checked_mul(price.0) {
                Ok(new_user_borrowed_value)
            } else {
                Err(FinanceError::UserBorrowedDeltaValueOverflow)
            }?;
            if let Some(new_user_total_borrowed_value) = base_user_borrowed_value.checked_add(new_user_borrowed_value) {
                Ok(NewUserTotalBorrowedValue(new_user_total_borrowed_value))
            } else {
                Err(FinanceError::UserTotalBorrowedValueTooHigh)
            }
        }

        fn get_rate(&self, token: &impl Token) -> Result<Option<Rate>, FinanceError> {
            let invested = if let Some(invested) = self.invested.get(token.id()) {
                invested
            } else {
                return Ok(None);
            };
            let borrowed = if let Some(borrowed) = self.borrowed.get(token.id()) {
                borrowed
            } else {
                return Ok(None);
            };
            let standard_rate: U128 = if let Some(standard_rate) = self.standard_rates.get(token.id()) {
                standard_rate.into()
            } else {
                return Ok(None);
            };
            if let Some(full_rate) = standard_rate.full_mul(invested.into()).checked_div(borrowed.into()) {
                match TryInto::<U128>::try_into(full_rate) {
                    Ok(rate) => Ok(Some(Rate(rate.as_u128()))),
                    Err(_) => Err(FinanceError::RateDoesNotFitImpossible)
                }
            } else {
                return Ok(None);
            }
        }

        fn calculate_interest(&self, rate: &Option<Rate>, new_price_updated_at: &Option<NewPriceUpdatedAt>) -> Result<Option<Interest>, FinanceError> {
            let time_delta = if let Some(new_price_updated_at) = new_price_updated_at {
                if let Some(time_delta) = new_price_updated_at.0.checked_sub(new_price_updated_at.2) {
                    Ok(time_delta)
                } else {
                    Err(FinanceError::TimeDeltaOverflowImpossible)
                }
            } else {
                return Ok(None);
            }?;
            let accumulated_rate = if let Some(rate) = rate {
                if let Some(accumulated_rate) = rate.0.checked_mul(time_delta.into()) {
                    Ok(accumulated_rate)
                } else {
                    Err(FinanceError::AccumulatedRateOverflow)
                }
            } else {
                return Ok(None);
            }?;
        }

        #[ink(message)]
        pub fn deposit(&mut self, token: AccountId, amount: u128) -> Result<(), FinanceError> {
            let user = &self.caller();
            let token = &self.enabled_token(token)?;
            let price = &self.up_to_date_price(token, user)?;
            let new_user_balance = self.new_user_balance_after_deposit(token, user, amount)?;
            let new_balance = self.new_token_balance_after_deposit(token, amount)?;
            let new_user_total_balance = self.new_user_total_balance_after_deposit(user, amount)?;
            let new_user_total_balance_value = self.updated_user_total_balance_value(user, price, &new_user_balance)?;
            
            self.set_user_balance(token, user, new_user_balance);
            self.set_token_balance(token, new_balance);
            self.set_user_total_balance(user, new_user_total_balance);
            self.set_user_total_balance_value(user, new_user_total_balance_value);
            
            Ok(())
        }
        
        #[ink(message)]
        pub fn withdraw(&mut self, token: AccountId, amount: u128) -> Result<(), FinanceError> {
            let user = &self.caller();
            let token = &self.withdraw_only_token(token);
            let price = &self.up_to_date_price(token, user)?;
            let new_user_total_balance = self.new_user_total_balance_after_withdraw(user, amount)?;
            let new_balance = self.new_token_balance_after_withdraw(token, amount)?;
            let new_user_balance = self.new_user_balance_after_withdraw(token, user, amount)?;
            let new_user_total_balance_value = self.updated_user_total_balance_value(user, price, &new_user_balance)?;
            
            self.set_token_balance(token, new_balance);
            self.set_user_balance(token, user, new_user_balance);
            self.set_user_total_balance(user, new_user_total_balance);
            self.set_user_total_balance_value(user, new_user_total_balance_value);
            
            Ok(())
        }
        
        #[ink(message)]
        pub fn invest(&mut self, token: AccountId, amount: u128) -> Result<(), FinanceError> {
            let user = &self.caller();
            let token = &self.enabled_token(token)?;
            let price = &self.up_to_date_price(token, user)?;
            
            let new_user_total_invested = self.new_user_total_invested_after_invest(user, amount)?;
            let new_user_invested = self.new_user_invested_after_invest(token, user, amount)?;
            let new_invested = self.new_token_invested_after_invest(token, amount)?;
            let new_user_total_invested_value = self.updated_user_total_invested(user, price, &new_user_invested)?;
            
            let new_balance = self.new_token_balance_after_withdraw(token, amount)?;
            let new_user_balance = self.new_user_balance_after_withdraw(token, user, amount)?;
            let new_user_total_balance = self.new_user_total_balance_after_withdraw(user, amount)?;
            let new_user_total_balance_value = self.updated_user_total_balance_value(user, price, &new_user_balance)?;
            
            self.set_token_balance(token, new_balance);
            self.set_user_balance(token, user, new_user_balance);
            self.set_user_total_balance(user, new_user_total_balance);
            self.set_user_total_balance_value(user, new_user_total_balance_value);
            
            self.set_user_invested(token, user, new_user_invested);
            self.set_token_invested(token, new_invested);
            self.set_user_total_invested(user, new_user_total_invested);
            self.set_user_total_invested_value(user, new_user_total_invested_value);
            
            Ok(())
        }
        
        #[ink(message)]
        pub fn redeposit(&mut self, token: AccountId, amount: u128) -> Result<(), FinanceError> {
            let user = &self.caller();
            let token = &self.redeposit_only_token(token);
            let price = &self.up_to_date_price(token, user)?;
            
            let new_invested = self.new_token_invested_after_redeposit(token, amount)?;
            let new_user_invested = self.new_user_invested_after_redeposit(token, user, amount)?;
            let new_user_total_invested = self.new_user_total_invested_after_redeposit(user, amount)?;
            let new_user_total_invested_value = self.updated_user_total_invested(user, price, &new_user_invested)?;
            
            let new_balance = self.new_token_balance_after_deposit(token, amount)?;
            let new_user_balance = self.new_user_balance_after_deposit(token, user, amount)?;
            let new_user_total_balance = self.new_user_total_balance_after_deposit(user, amount)?;
            let new_user_total_balance_value = self.updated_user_total_balance_value(user, price, &new_user_balance)?;
            
            self.set_token_balance(token, new_balance);
            self.set_user_balance(token, user, new_user_balance);
            self.set_user_total_balance(user, new_user_total_balance);
            self.set_user_total_balance_value(user, new_user_total_balance_value);
            
            self.set_user_invested(token, user, new_user_invested);
            self.set_token_invested(token, new_invested);
            self.set_user_total_invested(user, new_user_total_invested);
            self.set_user_total_invested_value(user, new_user_total_invested_value);
            
            Ok(())
        }
        
        #[ink(message)]
        pub fn borrow(&mut self, token: AccountId, amount: u128) -> Result<(), FinanceError> {
            let user = &self.caller();
            let token = &self.enabled_token(token)?;
            let price = &self.up_to_date_price(token, user)?;
            
            let new_user_total_borrowed = self.new_user_total_borrowed_after_borrow(user, amount)?;
            let new_borrowed = self.new_token_borrowed_after_borrow(token, amount)?;
            let new_user_borrowed = self.new_user_borrowed_after_borrow(token, user, amount)?;
            let new_user_total_borrowed_value = self.updated_user_total_borrowed(user, price, &new_user_borrowed)?;
            
            self.set_token_borrowed(token, new_borrowed);
            self.set_user_borrowed(token, user, new_user_borrowed);
            self.set_user_total_borrowed(user, new_user_total_borrowed);
            self.set_user_total_borrowed_value(user, new_user_total_borrowed_value);
            
            Ok(())
        }
        
        #[ink(message)]
        pub fn redeem(&mut self, token: AccountId, amount: u128) -> Result<(), FinanceError> {
            let user = &self.caller();
            let token = &self.redeem_only_token(token);
            let price = &self.up_to_date_price(token, user)?;
            
            let new_borrowed = self.new_token_borrowed_after_redeem(token, amount)?;
            let new_user_borrowed = self.new_user_borrowed_after_redeem(token, user, amount)?;
            let new_user_total_borrowed = self.new_user_total_borrowed_after_redeem(user, amount)?;
            let new_user_total_borrowed_value = self.updated_user_total_borrowed(user, price, &new_user_borrowed)?;
            
            self.set_token_borrowed(token, new_borrowed);
            self.set_user_borrowed(token, user, new_user_borrowed);
            self.set_user_total_borrowed(user, new_user_total_borrowed);
            self.set_user_total_borrowed_value(user, new_user_total_borrowed_value);

            Ok(())
        }

        #[ink(message)]
        pub fn update_price(&mut self, token: AccountId, user: AccountId, price: u128) -> Result<(), FinanceError> {
            let oracle = &self.oracle_caller()?;
            let user = &self.forwarded_user(user, oracle);
            let block = &self.block_number();
            let token = &self.supported_token(token)?;
            let new_updated_at = &self.new_updated_at(block);
            let new_price_updated_at = &self.new_price_updated_at(token, block, price, new_updated_at);

            let new_user_updated_at = &self.new_user_updated_at(user, block, new_updated_at);
            let new_user_unpriced_balance = self.new_user_unpriced_balance(token, user, new_user_updated_at)?;
            let new_user_unpriced_invested = self.new_user_unpriced_invested(token, user, new_user_updated_at)?;
            let new_user_unpriced_borrowed = self.new_user_unpriced_borrowed(token, user, new_user_updated_at)?;
            let new_user_total_balance_value = self.new_user_total_balance_value(token, user, price, new_user_updated_at)?;
            let new_user_total_invested_value = self.new_user_total_invested_value(token, user, price, new_user_updated_at)?;
            let new_user_total_borrowed_value = self.new_user_total_borrowed_value(token, user, price, new_user_updated_at)?;

            self.set_updated_at(new_updated_at, oracle);
            self.set_price_updated_at(token, new_price_updated_at, oracle);
            self.set_user_updated_at(user, new_user_updated_at, oracle);

            self.set_user_unpriced_balance(user, new_user_unpriced_balance);
            self.set_user_unpriced_invested(user, new_user_unpriced_invested);
            self.set_user_unpriced_borrowed(user, new_user_unpriced_borrowed);

            self.set_user_total_balance_value(user, new_user_total_balance_value);
            self.set_user_total_invested_value(user, new_user_total_invested_value);
            self.set_user_total_borrowed_value(user, new_user_total_borrowed_value);

            Ok(())
        }

        #[ink(message)]
        pub fn disable(&mut self, token: AccountId) -> Result<(), FinanceError> { 
            let admin = &self.admin_caller()?;

            self.set_token(&token, admin, false);
            Ok(())
        }

        #[ink(message)]
        pub fn enable(&mut self, token: AccountId) -> Result<(), FinanceError> { 
            let admin = &self.admin_caller()?;

            self.set_token(&token, admin, true);
            Ok(())
        }

        #[ink(message)]
        pub fn balance(&self, token: AccountId) -> u128 {
            if let Some(balance) = self.balances.get(token) {
                balance
            } else {
                0
            }
        }

        #[ink(message)]
        pub fn user_balance(&self, token: AccountId, user: AccountId) -> u128 {
            if let Some(user_balance) = self.user_balances.get((token, user)) {
                user_balance
            } else {
                0
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use core::cmp::Ordering;

        use super::*;

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
            match finance.enable(btc) {
                Err(FinanceError::CallerIsNotAdmin) => Ok(()),
                _ => e("Enable should fail if caller is not admin"),
            }?;

            set_caller(admin);
            finance.enable(btc)?;

            match finance.balance(btc).cmp(&0) {
                Ordering::Equal => Ok(()),
                _ => e("Token balance should be 0, before any deposit occurs"),
            }?;

            set_caller(user);
            finance.deposit(btc, u128::MAX)?;

            match finance.balance(btc).cmp(&u128::MAX) {
                Ordering::Equal => Ok(()),
                _ => e("Token balance should be MAX, after depositing MAX"),
            }?;

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
}