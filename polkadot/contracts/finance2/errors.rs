#[ink::scale_derive(Encode, Decode, TypeInfo)]
pub enum LAssetError {
    DepositOverflow,
    WithdrawOverflow,
    MintOverflow,
    BurnOverflow,
    BurnTooMuch,
    BorrowOverflow,
    RepayOverflow,
    CollateralValueTooLow,
}
