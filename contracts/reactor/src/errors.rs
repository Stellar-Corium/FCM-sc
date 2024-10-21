use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum ContractErrors {
    AlreadyDiscovered = 0,
    NonDiscovered = 1,
    NoMoreSupplyAvailable = 2,
    ProvidedHashIsInvalid = 3,
    ProvidedDifficultyIsInvalid = 4,
    MessageIsTooLarge = 5,
    MintedFCMPaymentFailed = 6,
    TheMineWasNuked = 7,
    NotTheFinder = 8,
    NothingToWithdraw = 9,
    StakeIsStillHot = 10,
    NotEnoughStaked = 11,
}
