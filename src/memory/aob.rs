use std::iter;
use anyhow::Result;
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Memory::{VirtualQuery, MEMORY_BASIC_INFORMATION, MEM_COMMIT, PAGE_EXECUTE_READ, PAGE_EXECUTE_READWRITE, PAGE_EXECUTE_WRITECOPY, PAGE_GUARD, PAGE_READONLY, PAGE_READWRITE, PAGE_WRITECOPY};

fn is_module_readable(mbi: &MEMORY_BASIC_INFORMATION) -> bool {
    const READ: u32 = PAGE_READONLY.0
        | PAGE_READWRITE.0
        | PAGE_WRITECOPY.0
        | PAGE_EXECUTE_READ.0
        | PAGE_EXECUTE_READWRITE.0
        | PAGE_EXECUTE_WRITECOPY.0;

    mbi.State == MEM_COMMIT
        && mbi.Protect.0 & PAGE_GUARD.0 == 0
        && mbi.Protect.0 & READ != 0
}

fn get_module_size(module_base: usize) -> usize {
    // e_lfanew at 0x3C -> PE header; SizeOfImage lives at optional header + 0x38
    unsafe {
        let e_lfanew = *((module_base + 0x3C) as *const u32) as usize;
        *((module_base + e_lfanew + 0x18 + 0x38) as *const u32) as usize
    }
}

pub fn get_aob_bytes_address(module: &str, bytes: String) -> Result<usize> {
    let module_base;
    unsafe  {
        module_base = GetModuleHandleW(PCWSTR(HSTRING::from(module).as_ptr()))?.0 as usize;
    }

    let module_end = module_base + get_module_size(module_base);
    let pattern = aobscan::PatternBuilder::from_ida_style(&*bytes)?
        .with_all_threads()
        .build();

    let mut current = module_base;
    while current < module_end {
        let mut mbi = MEMORY_BASIC_INFORMATION::default();

        unsafe {
            if VirtualQuery(
                Some(current as *const _),
                &mut mbi as *mut MEMORY_BASIC_INFORMATION,
                size_of::<MEMORY_BASIC_INFORMATION>(),
            ) == 0 {
                return Err(anyhow::anyhow!("VirtualQuery failed"));
            }
        }

        let region_base = mbi.BaseAddress as usize;
        let region_size = mbi.RegionSize.min(module_end - region_base);

        if is_module_readable(&mbi) {
            let region_bytes: &[u8] = unsafe {
                std::slice::from_raw_parts(region_base as *const u8, region_size)
            };

            let mut found_offset = None;
            pattern.scan(region_bytes, |offset| {
                found_offset = Some(offset);
                false
            });

            if let Some(offset) = found_offset {
                return Ok(region_base + offset);
            }
        }

        current = region_base + mbi.RegionSize;
    }

    Err(anyhow::anyhow!("Did not find AOB"))
}
