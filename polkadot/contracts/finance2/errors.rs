use traits::psp22::PSP22Error;

#[ink::scale_derive(Encode, Decode, TypeInfo)]
pub enum LAssetError {
    DepositOverflow,
    DepositTransferFailed(PSP22Error),
    FirstDepositRequiresGasCollateral,

    WithdrawOverflow,
    WithdrawTransferFailed(PSP22Error),
    WithdrawWithoutDeposit,
    CollateralValueTooLowAfterWithdraw,

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

    IncreaseCashTransferFailed(PSP22Error),
    IncreaseCashOverflow,

    LiquidateForNothing,
    LiquidateCollateralOverflow,
    LiquidateTooMuch,
    LiquidateTooEarly,
    LiquidateTransferFailed(PSP22Error),

    RepayOverflow,
    RepayWithoutBorrow,
    RepayTransferFailed(PSP22Error),
    RepayInsufficientCash,
    RepayInsufficientInternalCash,
    RepayCashOverflow,

    SetPriceUnathorized,
}
