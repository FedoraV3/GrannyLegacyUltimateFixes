use std::ffi::c_void;
use anyhow::Result;
use windows::Win32::System::Memory::{VirtualProtect, PAGE_EXECUTE_READWRITE, PAGE_PROTECTION_FLAGS};

pub unsafe fn patch_bytes(address: usize, size: usize, bytes: &[u8]) -> Result<()> {

    let mut old = PAGE_PROTECTION_FLAGS::default();
    unsafe {
        VirtualProtect(
            address as *const c_void,
            size,
            PAGE_EXECUTE_READWRITE,
            &mut old as *mut PAGE_PROTECTION_FLAGS
        )?;
    }

    unsafe {
        std::ptr::copy_nonoverlapping(
            bytes.as_ptr(),
            address as *mut u8,
            size
        );
    }

    {
        let mut shit = PAGE_PROTECTION_FLAGS::default();
        unsafe {
            VirtualProtect(
                address as *const c_void,
                size,
                old,
                &mut shit as *mut PAGE_PROTECTION_FLAGS
            )?;
        }
    }

    Ok(())
}