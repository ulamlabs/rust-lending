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

    MintLiquidityOverflow,
    MintOverflow,
    MintTransferFailed(PSP22Error),

    BurnOverflow,
    BurnTooMuch,
    BurnTransferFailed(PSP22Error),
    
    BorrowOverflow,
    FirstBorrowRequiresGasCollateral,
    BorrowTransferFailed(PSP22Error),
    CollateralValueTooLowAfterBorrow,
    BorrowWhileDepositingNotAllowed,

    DepositCashTransferFailed(PSP22Error),
    DepositCashOverflow,

    WithdrawCashTransferFailed(PSP22Error),

    LiquidateForNothing,
    LiquidateCollateralOverflow,
    LiquidateTooMuch,
    LiquidateTooEarly,
    LiquidateTransferFailed(PSP22Error),
    LiquidateGasTransferFailed,

    RepayOverflow,
    RepayWithoutBorrow,
    RepayTransferFailed(PSP22Error),
    RepayInsufficientCash,
    RepayInsufficientInternalCash,
    RepayCashOverflow,
    RepayGasTransferFailed,

    SetPriceUnathorized,

    SetParamsUnathorized,

    TryCloseTransferFailed,

    #[cfg(test)]
    TestError(String),
}
