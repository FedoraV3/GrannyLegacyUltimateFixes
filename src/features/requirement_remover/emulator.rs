use anyhow::{Context, Result};
use il2cpp_bridge_rs::api;
use retour::RawDetour;
use windows::core::w;
use windows::Win32::Foundation::FreeLibrary;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use crate::memory::aob::get_aob_bytes_address;
use crate::memory::memory_constants::{GET_LOGGED_ON_CALL, GET_LOGGED_ON_CALL_AOB_SIZE, GET_LOGGED_ON_NEW_BYTES, GET_LOGGED_ON_NEW_BYTES_U8};
use crate::memory::patch::patch_bytes;

pub(crate) fn install_emulator() -> Result<()> {
    unsafe {
        FreeLibrary(GetModuleHandleW(w!("steam_api64.dll"))?)?;
    }

    let game_input_manager = api::cache::csharp()
        .class("GameInputManager")
        .context("Failed to get GameInputManager")?;

    let game_input_manager_start_method = game_input_manager
        .method("Start")
        .context("Failed to get GameInputManager$$Start")?;


    println!("{:#x}", game_input_manager_start_method.va + 0xC0);

    unsafe {
        patch_bytes((game_input_manager_start_method.va + 0xC0) as usize, GET_LOGGED_ON_CALL_AOB_SIZE, &GET_LOGGED_ON_NEW_BYTES_U8)?;
    }

    Ok(())
}