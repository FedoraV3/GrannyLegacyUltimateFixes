pub mod aob;
pub mod il2cpp_symbols;
pub mod patch;
pub mod resolver;

//constants
pub mod memory_constants {
    pub mod offsets {
        // convert to 64 bit by sending 64 bit numbers back
        pub static UNITY_ENGINE_RANDOM_RANDOM_RANGE_INT: usize = 0x72AB30;
    }

    pub mod generics {
        pub const INIT_METADATA_SLOT: usize = 0x138670;

        pub const HASHSET_CLEAR: usize = 0x3363B0;
        pub const HASHSET_ADD: usize = 0x336260;
        pub const HASHSET_CONTAINS: usize = 0x336410;
        pub const DICTIONARY_CLEAR: usize = 0x318CD0;
        pub const DICTIONARY_SET_ITEM: usize = 0x31DD00;
        pub const DICTIONARY_GET_ITEM: usize = 0x31D9A0;
        pub const DICTIONARY_TRY_GET_VALUE: usize = 0x31C170;
        pub const LIST_CLEAR: usize = 0x32D0;
        pub const GET_COMPONENT: usize = 0x2B9A30;
        pub const SHUFFLE: usize = 0x2D9140;

        pub const MI_SHUFFLE_GAMEOBJECT: usize = 0xC435A0;
        pub const MI_GET_COMPONENT_ITEM_SEED_DATA: usize = 0xC36B48;
        pub const MI_HASHSET_STRING_CLEAR: usize = 0xC4DA58;
        pub const MI_HASHSET_STRING_ADD: usize = 0xC4DA20;
        pub const MI_HASHSET_STRING_CONTAINS: usize = 0xC4DA90;
        pub const MI_HASHSET_TRANSFORM_CLEAR: usize = 0xC4DDD8;
        pub const MI_HASHSET_TRANSFORM_ADD: usize = 0xC4DDA0;
        pub const MI_HASHSET_TRANSFORM_CONTAINS: usize = 0xC4DE10;
        pub const MI_DICTIONARY_STRING_BOOL_CLEAR: usize = 0xC3FB70;
        pub const MI_DICTIONARY_STRING_BOOL_SET_ITEM: usize = 0xC3FC30;
        pub const MI_DICTIONARY_STRING_BOOL_GET_ITEM: usize = 0xC3FBF0;
        pub const MI_DICTIONARY_STRING_GAMEOBJECT_TRY_GET_VALUE: usize = 0xC400B0;
        pub const MI_LIST_TRANSFORM_CLEAR: usize = 0xC34328;
    }

    pub static CHARACTER_LIMIT_PARAMETER_AOB: &str = "00 45 33 C0 BA 09 00 00 00 E8 A2 EA 42 00 48 8B 4B 38 45 33 C0 48 8B 7B 28 33 D2 E8 30 DC 4D 00";
    pub static CHARACTER_LIMIT_PARAMETER_AOB_U8: [u8; 32] = [0x00, 0x45, 0x33, 0xC0, 0xBA, 0x09, 0x00, 0x00, 0x00, 0xE8, 0xA2, 0xEA, 0x42, 0x00, 0x48, 0x8B, 0x4B, 0x38, 0x45, 0x33, 0xC0, 0x48, 0x8B, 0x7B, 0x28, 0x33, 0xD2, 0xE8, 0x30, 0xDC, 0x4D, 0x00];
    pub static CHARACTER_LIMIT_PARAMETER_AOB_SIZE: usize = 32;
    pub static STEAM_API_INIT_AOB: &str = "33 C9 E9 99 F2 FF FF";
    pub static STEAM_API_INIT_AOB_U8: [u8; 7] = [0x33, 0xC9, 0xE9, 0x99, 0xF2, 0xFF, 0xFF];
    pub static STEAM_API_INIT_AOB_SIZE: usize = 7;
    pub static GET_LOGGED_ON_CALL_AOB: &str = "E8 9B DF 09 00";
    pub static GET_LOGGED_ON_CALL_AOB_U8: [u8; 5] = [0xE8, 0x9B, 0xDF, 0x09, 0x00];
    pub static GET_LOGGED_ON_CALL_AOB_SIZE: usize = 5;


    // for new bytes
    pub static CHARACTER_LIMIT_PARAMETER_NEW_BYTES: &str = "00 45 33 C0 BA 14 00 00 00 E8 A2 EA 42 00 48 8B 4B 38 45 33 C0 48 8B 7B 28 33 D2 E8 30 DC 4D 00";
    pub static CHARACTER_LIMIT_PARAMETER_NEW_BYTES_U8: [u8; 32] = [0x00, 0x45, 0x33, 0xC0, 0xBA, 0x14, 0x00, 0x00, 0x00, 0xE8, 0xA2, 0xEA, 0x42, 0x00, 0x48, 0x8B, 0x4B, 0x38, 0x45, 0x33, 0xC0, 0x48, 0x8B, 0x7B, 0x28, 0x33, 0xD2, 0xE8, 0x30, 0xDC, 0x4D, 0x00];
    pub static STEAM_API_INIT_NEW_BYTES: &str = "90 90 EB 0C 90 90 90";
    pub static STEAM_API_INIT_NEW_BYTES_U8: [u8; 7] = [0x90, 0x90, 0xEB, 0x0C, 0x90, 0x90, 0x90];
    pub static GET_LOGGED_ON_NEW_BYTES: &str = "B0 01 90 90 90";
    pub static GET_LOGGED_ON_NEW_BYTES_U8: [u8; 5] = [0xB0, 0x01, 0x90, 0x90, 0x90];

    pub static GET_LOGGED_ON_CALL: u64 = 0xC0;
}

// 48 8B 0D 5D
// 48 8B 5E 40