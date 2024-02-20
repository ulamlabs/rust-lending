use crate::psp22::PSP22Error;

#[ink::scale_derive(Encode, Decode, TypeInfo)]
pub enum LAssetError {
    DepositOverflow,
    DepositTransferFailed(PSP22Error),
    FirstDepositRequiresGasCollateral,

    WithdrawOverflow,
    WithdrawTransferFailed(PSP22Error),

    MintOverflow,
    BurnOverflow,
    BurnTooMuch,
    BorrowOverflow,
    RepayOverflow,
    CollateralValueTooLow,
}
