use crate::psp22::PSP22Error;
use ink::prelude::string::String;

#[ink::scale_derive(Encode, Decode, TypeInfo)]
pub enum FlashLoanPoolError {
    TakeCashUnauthorized,
    TakeCashFailed(PSP22Error),
}

#[ink::scale_derive(Encode, Decode, TypeInfo)]
pub enum FlashLoanReceiverError {
    Error,
    CustomError(String)
}
