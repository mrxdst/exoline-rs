use num_enum::{FromPrimitive, IntoPrimitive};

/// Error codes returned from the device.
#[repr(u8)]
#[derive(Debug, Clone, Copy, IntoPrimitive, FromPrimitive)]
pub enum EXOlineException {
    WrongType = 1,
    WrongSLn = 2,
    WrongDLn = 3,
    WrongTLn = 4,
    DPacNotPresent = 5,
    DPacExists = 6,
    DPacNotPrep = 7,
    VPacUsed = 8,
    TaskNotPresent = 9,
    TaskExists = 10,
    WrongLoadOrder = 11,
    INSTNotAllowed = 12,
    KILLTNotAllowed = 13,
    TaskIsRunning = 14,
    TaskNotRunning = 15,
    TaskNotInstalled = 16,
    STEPTNotAllowed = 17,
    TextExists = 18,
    TextNotPrepared = 19,
    MemoryFull = 20,
    TextEmpty = 21,
    TextTruncated = 22,
    AccessTooLow = 23,
    AccessTooHigh = 24,
    ParamIllegal = 25,
    WrongKey = 26,
    NoAccess = 28,
    TooBigMaxLength = 29,
    ProcUsedByTask = 32,
    OutOfTextSpace = 33,
    NotInStepMode = 34,
    DPacEmpty = 35,
    IllegalCell = 37,
    IllegalCommand = 38,
    IllegalMessageLength = 39,
    AddressOutsideRange = 41,
    #[num_enum(catch_all)]
    Unknown(u8),
}

impl PartialEq for EXOlineException {
    fn eq(&self, other: &Self) -> bool {
        u8::from(*self) == u8::from(*other)
    }
}
