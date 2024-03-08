use ink::prelude::string::String;

#[ink::scale_derive(Encode, Decode, TypeInfo)]
pub enum FlashLoanReceiverError {
    Error,
    CustomError(String)
}

#[ink::scale_derive(Encode, Decode, TypeInfo)]
pub enum FlashLoanError {
    TransferFailed(traits::psp22::PSP22Error),
    ReceiverFailed(FlashLoanReceiverError),
    TakeCashFailed(finance2::errors::TakeCashError),
    Overflow,
    Unathorized,
}

#[ink::scale_derive(Encode, Decode, TypeInfo)]
pub enum AdminError {
    AddAssetUnauthorized,
    PushPriceUnauthorized,
    PushParamsUnauthorized,
}
