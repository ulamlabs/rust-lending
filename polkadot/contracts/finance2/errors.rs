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
    MintOverflow,
    MintTransferFailed(PSP22Error),

    BurnOverflow,
    BurnTooMuch,
    BurnTransferFailed(PSP22Error),
    
    BorrowOverflow,
    BorrowableOverflow,
    FirstBorrowRequiresGasCollateral,
    BorrowTransferFailed(PSP22Error),

    RepayOverflow,
    RepayWithoutBorrow,
    CollateralValueTooLow,
}
