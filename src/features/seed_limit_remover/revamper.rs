use anyhow::{Context, Result};
use il2cpp_bridge_rs::api::cache;
use retour::RawDetour;
use crate::features::seed_limit_remover::random::ue_random_range_int;

pub unsafe fn install() -> Result<()> {

    // grab the rva of UnityEngine_Random__RandomRangeInt
    let rnd_class = cache::assembly("GameAssembly").context("Failed to get Assemblies")?
        .class("Random").context("Could not get ")?;

    let random_rangeint_rva = rnd_class.method("RandomRangeInt").context("Could not get UnityEngine_Random__RandomRangeInt")?.rva;

    let random_hook;
    unsafe {
        random_hook = RawDetour::new(ue_random_range_int as *const (), random_rangeint_rva as *const ())?;
    }

    // We do not need any original function as we have basically revamped the entire way the function works, by making int64 usable
    unsafe {
       random_hook.enable()?;
    }
    
    Ok(())
}