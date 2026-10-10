use anyhow::{Result, anyhow, Context};
use windows::Win32::System::Memory::{VirtualAlloc, MEM_COMMIT, PAGE_EXECUTE_READWRITE};
use std::ffi::c_void;
use windows::core::{s, w};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use crate::memory::aob::get_aob_bytes_address;
use crate::memory::memory_constants::{STEAM_API_INIT_AOB, STEAM_API_INIT_NEW_BYTES_U8, STEAM_API_INIT_AOB_SIZE};
use crate::memory::patch::patch_bytes;

pub unsafe fn remove_granny_steam_requirement() -> Result<()> {
    // TODO: Check if using GetProcAddress works and use it instead

    let steam_api_64;
    unsafe {
        steam_api_64 = GetModuleHandleW(w!("steam_api64.dll"))?;
    }

    let steamapi_init_func;
    unsafe {
        steamapi_init_func = GetProcAddress(steam_api_64, s!("SteamAPI_Init")).context("Failed to get SteamAPI_Init")? as usize;
        patch_bytes(steamapi_init_func, STEAM_API_INIT_AOB_SIZE, &STEAM_API_INIT_NEW_BYTES_U8)?;
    }

    Ok(())
}