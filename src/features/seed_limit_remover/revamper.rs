use anyhow::{Context, Result};
use il2cpp_bridge_rs::api::{Thread, cache};
use retour::RawDetour;
use crate::features::seed_limit_remover::{hooks, remover};

fn detour(class: &str, method: &str, argc: usize, hook: *const (), set_original: fn(usize)) -> Result<()> {
    let target = cache::assembly("Assembly-CSharp").context("Failed to get Assembly-CSharp")?
        .class(class).with_context(|| format!("Could not get {class}"))?
        .method((method, argc)).with_context(|| format!("Could not get {class}::{method}"))?;

    let detour = unsafe { RawDetour::new(target.function as *const (), hook)? };

    set_original(detour.trampoline() as *const () as usize);
    unsafe {
        detour.enable()?;
    }

    Box::leak(Box::new(detour));

    Ok(())
}

pub fn install_hooks() -> Result<()> {
    let _attached = Thread::attach(true);

    remover::set_seed_char_limit_higher()?;
    
    hooks::init()?;
    hooks::init_menu()?;

    detour("SeedManager", "GeneratePlacement", 0, hooks::generate_placement as *const (), hooks::set_original)?;

    detour("Menu_Seed", "Start", 0, hooks::menu_seed_start as *const (), hooks::set_original_menu_start)?;
    detour("Menu_Seed", "Update", 0, hooks::menu_seed_update as *const (), hooks::set_original_menu_update)?;
    detour("Menu_Seed", "ExitTypingModeAndSave", 0, hooks::menu_seed_exit_typing as *const (), hooks::set_original_menu_exit_typing)?;

    Ok(())
}
