use anyhow::Result;
use crate::memory::aob::get_aob_bytes_address;
use crate::memory::memory_constants::{CHARACTER_LIMIT_PARAMETER_AOB, CHARACTER_LIMIT_PARAMETER_AOB_SIZE, CHARACTER_LIMIT_PARAMETER_NEW_BYTES_U8};
use crate::memory::patch::patch_bytes;

pub fn remove_seed_limit() -> Result<()> {
    // same pattern as requirement remover

    let aob_address = get_aob_bytes_address("GameAssembly.dll", CHARACTER_LIMIT_PARAMETER_AOB.to_string())?;
    unsafe {
        patch_bytes(aob_address, CHARACTER_LIMIT_PARAMETER_AOB_SIZE, &CHARACTER_LIMIT_PARAMETER_NEW_BYTES_U8)?;
    }

    Ok(())
}