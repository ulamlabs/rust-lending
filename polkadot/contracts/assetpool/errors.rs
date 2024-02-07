use psp22::PSP22Error;

#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum AssetPoolError {
    /// PSP22Error from underlying asset or collateral asset.
    PSP22Error(PSP22Error),
    /// RatioCalculationError due to overflow or zero division
    RatioCalculationError,
    /// Non-manager tried to call an authorized function
    Unauthorized
}

impl From<PSP22Error> for AssetPoolError {
    fn from(err: PSP22Error) -> Self {
        AssetPoolError::PSP22Error(err)
    }
}

pub type AssetPoolResult = Result<(), AssetPoolError>;
