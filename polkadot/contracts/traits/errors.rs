use psp22::PSP22Error;


#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum FinanceError {
    InvalidAction,
    OldInvestedZeroImpossible,
    CumulativeBorrowRateOverflow,
    CumulativeInvestRateOverflow,
    UserBorrowedWithCumulativeZeroImpossible,
    UserBorrowedWithCumulativeOverflow,
    UserInvestedWithCumulativeOverflow,
    UserTotalBalanceValueEmptyImpossible,
    UserTotalBorrowedValueEmptyImpossible,
    BorrowHealthCheckFailed,
    WithdrawHealthCheckFailed,
    RedepositHealthCheckFailed,
    UnpricedBalanceOverflowImpossible,
    UnpricedInvestedOverflowImpossible,
    UnpricedBorrowedOverflowImpossible,
    UserBalanceValueTooHigh,
    UserInvestedValueTooHigh,
    UserBorrowedValueTooHigh,
    UserTotalBalanceValueTooHigh,
    UserTotalInvestedValueTooHigh,
    UserTotalBorrowedValueTooHigh,
    NothingToRepay,
    NothingToRepayForUser,
    NothingToRepayForUserTotal,
    UserTotalBorrowedNegativeDeltaImpossible,
    RepayTooMuch,
    RepayTooMuchForUser,
    RepayTooMuchForUserTotal,
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
    CallerIsNotToken,
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
    FullRateOverflow,
    BorrowedWithInterestOverflow,
    InvestedWithInterestOverflow,
    InterestOverflow,
    CalculatedInterestOverflowImpossible,
    NegativeInterestImpossible,

    #[cfg(any(feature = "std", test, doc))]
    Test(String)
}

#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum AssetPoolError {
    /// RatioCalculationError due to overflow or zero division
    RatioCalculationError,
}


#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum ContractError {
    AssetPoolError(AssetPoolError),
    FinanceError(FinanceError),
    PSP22Error(PSP22Error),
}

impl From<AssetPoolError> for ContractError {
    fn from(err: AssetPoolError) -> Self {
        ContractError::AssetPoolError(err)
    }
}

impl From<FinanceError> for ContractError {
    fn from(err: FinanceError) -> Self {
        ContractError::FinanceError(err)
    }
}

impl From<PSP22Error> for ContractError {
    fn from(err: PSP22Error) -> Self {
        ContractError::PSP22Error(err)
    }
}
