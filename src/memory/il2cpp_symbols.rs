use std::ffi::{CStr, c_char};

use anyhow::{Result, bail};
use il2cpp_bridge_rs::memory::symbol::cache_symbol;
use windows::core::{PCSTR, w};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};

// il2cpp-bridge-rs looks exports up in the exe on windows, so hand it GameAssembly.dll's instead
pub fn preload_il2cpp_symbols() -> Result<usize> {
    let module = unsafe { GetModuleHandleW(w!("GameAssembly.dll"))? };
    let base = module.0 as usize;

    unsafe {
        // e_lfanew -> PE header; export directory RVA lives at optional header + 0x70
        let e_lfanew = *((base + 0x3C) as *const u32) as usize;
        let export_rva = *((base + e_lfanew + 0x18 + 0x70) as *const u32) as usize;
        if export_rva == 0 {
            bail!("GameAssembly.dll has no export table");
        }

        let directory = base + export_rva;
        let count = *((directory + 0x18) as *const u32) as usize;
        let names = (base + *((directory + 0x20) as *const u32) as usize) as *const u32;

        let mut cached = 0;
        for i in 0..count {
            let name_ptr = (base + *names.add(i) as usize) as *const c_char;
            let Ok(name) = CStr::from_ptr(name_ptr).to_str() else { continue };
            if !name.starts_with("il2cpp_") {
                continue;
            }
            if let Some(function) = GetProcAddress(module, PCSTR(name_ptr as *const u8)) {
                cache_symbol(name, function as usize);
                cached += 1;
            }
        }
        Ok(cached)
    }
}
