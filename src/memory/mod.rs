pub mod aob;
pub mod patch;

//constants
pub mod memory_constants {
    pub static CHARACTER_LIMIT_PARAMETER_AOB: &str = "BA 09 00 00 00";
    pub static CHARACTER_LIMIT_PARAMETER_AOB_U8: [u8; 5] = [0xBA, 0x09, 0x00, 0x00, 0x00];
    pub static CHARACTER_LIMIT_PARAMETER_AOB_SIZE: usize = 5;
    pub static STEAM_API_INIT_AOB: &str = "33 C9 E9 99 F2 FF FF";
    pub static STEAM_API_INIT_AOB_U8: [u8; 7] = [0x33, 0xC9, 0xE9, 0x99, 0xF2, 0xFF, 0xFF];
    pub static STEAM_API_INIT_AOB_SIZE: usize = 7;

    // for new bytes
    pub static CHARACTER_LIMIT_PARAMETER_NEW_BYTES: &str = "BA FF FF FF 7F";
    pub static CHARACTER_LIMIT_PARAMETER_NEW_BYTES_U8: [u8; 5] = [0xBA, 0xFF, 0xFF, 0xFF, 0x7F];
    pub static STEAM_API_INIT_NEW_BYTES: &str = "90 90 EB 0C 90 90 90";
    pub static STEAM_API_INIT_NEW_BYTES_U8: [u8; 7] = [0x90, 0x90, 0xEB, 0x0C, 0x90, 0x90, 0x90];
}
