use crate::psp22::PSP22Error;
use ink::prelude::string::String;

#[derive(Debug)]
#[ink::scale_derive(Encode, Decode, TypeInfo)]
pub enum FlashLoanPoolError {
    TakeCashUnauthorized,
    TakeCashFeeOverflow,
    TakeCashOverflow,
    TakeCashFailed(PSP22Error),
}

#[ink::scale_derive(Encode, Decode, TypeInfo)]
pub enum FlashLoanReceiverError {
    Error,
    CustomError(String)
}
