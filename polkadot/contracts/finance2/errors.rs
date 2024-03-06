use traits::psp22::PSP22Error;

#[derive(Debug)]
#[ink::scale_derive(Encode, Decode, TypeInfo)]
pub enum LAssetError {
    DepositOverflow,
    DepositTransferFailed(PSP22Error),
    FirstDepositRequiresGasCollateral,
    DepositWhileBorrowingNotAllowed,

    WithdrawOverflow,
    WithdrawTransferFailed(PSP22Error),
    WithdrawWithoutDeposit,
    CollateralValueTooLowAfterWithdraw,
    WithdrawGasTransferFailed,

    MintOverflow,
    MintTransferFailed(PSP22Error),
    MintFeeOverflow,

    BurnOverflow,
    BurnTooMuch,
    BurnTransferFailed(PSP22Error),
    
    BorrowOverflow,
    BorrowFeeOverflow,
    FirstBorrowRequiresGasCollateral,
    BorrowTransferFailed(PSP22Error),
    CollateralValueTooLowAfterBorrow,
    BorrowWhileDepositingNotAllowed,

    DepositCashTransferFailed(PSP22Error),
    DepositCashOverflow,

    WithdrawCashTransferFailed(PSP22Error),

    LiquidateForNothing,
    LiquidateTotalRepaidOverflow,
    LiquidateRewardsOverflow,
    LiquidateRepaidCollateralOverflow,
    LiquidateCollateralOverflow,
    LiquidateTooMuch,
    LiquidateTooEarly,
    LiquidateTransferFailed(PSP22Error),
    LiquidateGasTransferFailed,

    RepayWithoutBorrow,
    RepayTransferFailed(PSP22Error),
    RepayCashOverflow,
    RepayGasTransferFailed,

    SetPriceUnathorized,

    SetParamsUnathorized,

    #[cfg(test)]
    TestError(String),
}
