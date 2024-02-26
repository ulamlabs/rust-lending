use crate::psp22::PSP22Error;
use ink::prelude::string::String;

#[ink::scale_derive(Encode, Decode, TypeInfo)]
pub enum LAssetError {
    CallerIsNotAdmin,

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
    LiquidateSelf,
    LiquidateInvalid,
    LiquidateTooMuch,
    LiquidateTooEarly,
    LiquidateCollateralOverflow,

    ForceRepayTransferFailed(PSP22Error),
    ForceRepayWithoutBorrow,

    IncreaseCashOverflow,
    IncreaseCashTransferFailed(PSP22Error),

    RepayNotWhitelisted,
    RepayInvalidCaller,

    /// Only the flash loan contract can call this endpoint
    FlashContractOnly,
    /// Take cash inside L-Asset Pool failed
    TakeCashFailed(PSP22Error),
    /// Flash loan failed
    FlashLoanTransferFailed(PSP22Error),
    FlashLoanFailed(FlashCalleeError),
    FlashLoanOverflow,
}


#[ink::scale_derive(Encode, Decode, TypeInfo)]
pub enum FlashCalleeError {
    Error,
    CustomError(String)
}
