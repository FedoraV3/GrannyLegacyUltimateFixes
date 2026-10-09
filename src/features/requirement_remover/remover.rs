use anyhow::{Result, anyhow};
use windows::Win32::System::Memory::{VirtualAlloc, MEM_COMMIT, PAGE_EXECUTE_READWRITE};
use std::ffi::c_void;
use crate::memory::aob::get_aob_bytes_address;
use crate::memory::memory_constants::{STEAM_API_INIT_AOB, STEAM_API_INIT_NEW_BYTES_U8, STEAM_API_INIT_AOB_SIZE};
use crate::memory::patch::patch_bytes;

pub unsafe fn remove_granny_steam_requirement() -> Result<()> {
    // TODO: Check if using GetProcAddress works and use it instead

    let steam_init_address = get_aob_bytes_address("steam_api64.dll", STEAM_API_INIT_AOB.to_string())?;
    unsafe {
        patch_bytes(steam_init_address, STEAM_API_INIT_AOB_SIZE, &STEAM_API_INIT_NEW_BYTES_U8)?;
    }

    Ok(())
}