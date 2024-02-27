#[ink::scale_derive(Encode, Decode, TypeInfo)]
pub enum FlashLoanError {
    TransferFailed(traits::psp22::PSP22Error),
    ReceiverFailed(traits::errors::FlashLoanReceiverError),
    TakeCashFailed(traits::errors::FlashLoanPoolError),
    Overflow,
    Unathorized,
}