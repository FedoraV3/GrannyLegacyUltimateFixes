use std::ffi::c_void;
use std::thread::sleep;
use std::time::Duration;
use windows::core::s;
use windows::Win32::System::Console::AllocConsole;
use windows::Win32::System::LibraryLoader::GetModuleHandleA;
use windows::Win32::System::Threading::{CreateThread, THREAD_CREATION_FLAGS};
use crate::features::requirement_remover::remover::remove_granny_steam_requirement;
use crate::features::seed_limit_remover::remover::remove_seed_limit;

pub mod memory;
pub mod features;

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
unsafe extern "C" fn InitializeASI() {
    let is_crash_handler = std::env::current_exe()
        .ok()
        .and_then(|path| path.file_stem().map(|stem| stem.to_string_lossy().to_ascii_lowercase()))
        .is_some_and(|stem| stem.starts_with("unitycrashhandler"));

    if is_crash_handler {
        return;
    }

    unsafe {
        CreateThread(
            None,
            0,
            Some(ultimate_thread),
            None,
            THREAD_CREATION_FLAGS(0),
            None
        ).unwrap();
    }
}

unsafe extern "system" fn ultimate_thread(arg: *mut c_void) -> u32 {
    // waiting until the gameassembly.dll is lodaed
    let _ga = loop {
        if let Ok(_h) = unsafe { GetModuleHandleA(s!("GameAssembly.dll")) } {
            break;
        }
        sleep(Duration::from_millis(100));
    };

    unsafe {
        AllocConsole().unwrap();
        remove_granny_steam_requirement().unwrap();
        println!("Loaded granny steam requirement remover");

        remove_seed_limit().unwrap();
        println!("Loaded granny seed limit remover");
    }


    1
}