use crate::psp22::PSP22Error;

#[ink::scale_derive(Encode, Decode, TypeInfo)]
pub enum LAssetError {
    DepositOverflow,
    DepositTransferFailed(PSP22Error),
    FirstDepositRequiresGasCollateral,

    WithdrawOverflow,
    WithdrawTransferFailed(PSP22Error),
    WithdrawWithoutDeposit,

    MintLiquidityOverflow,
    MintSharesOverflow,
    MintOverflow,
    MintTransferFailed(PSP22Error),

    BurnOverflow,
    BurnTooMuch,
    BurnTransferFailed(PSP22Error),
    
    BorrowOverflow,
    BorrowSharesOverflow,
    BorrowableOverflow,
    FirstBorrowRequiresGasCollateral,
    BorrowTransferFailed(PSP22Error),

    RepayOverflow,
    RepayWithoutBorrow,
    RepayTransferFailed(PSP22Error),
    RepayInsufficientCash,
    RepayCashOverflow,

    CollateralValueTooLow,

    LiquidateTransferFailed(PSP22Error),
    LiquidateApproveFailed(PSP22Error),
    LiquidateForNothing,

    ForceRepayTransferFailed(PSP22Error),
    ForceRepayWithoutBorrow,
}
