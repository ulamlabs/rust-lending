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
    LiquidateTooMuch,
    LiquidateTooEarly,
    LiquidateTransferFailed(PSP22Error),

    RepayWithoutBorrow,
    RepayTransferFailed(PSP22Error),
    RepayCashOverflow,

    SetPriceUnathorized,

    SetParamsUnathorized,
}
