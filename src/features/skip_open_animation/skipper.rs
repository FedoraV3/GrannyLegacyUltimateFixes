use std::ffi::c_void;

use anyhow::{Result, anyhow, bail};
use il2cpp_bridge_rs::api::{Thread, cache};
use il2cpp_bridge_rs::structs::{Class, Method};
use windows::Win32::System::Diagnostics::Debug::{FlushInstructionCache, RtlLookupFunctionEntry};
use windows::Win32::System::Threading::GetCurrentProcess;

use crate::memory::patch::patch_bytes;

const PLAY_AOB: &str = "48 8B 15 ?? ?? ?? ?? 45 33 C0 48 89 5C 24 38 E8 ?? ?? ?? ??";
const PLAY_CALL: usize = 15;
const PLAY_NOPS: [(usize, usize); 3] = [(0, 7), (7, 3), (PLAY_CALL, 5)];

const WAIT_AOB: &str = "F3 0F 10 0D ?? ?? ?? ?? 45 33 C0 48 8B C8";
const WAIT_ZERO: [u8; 8] = [0x0F, 0x57, 0xC9, 0x90, 0x90, 0x90, 0x90, 0x90];

const UNW_FLAG_CHAININFO: u8 = 0x4;

fn class(assembly: &str, name: &str) -> Result<Class> {
    cache::assembly(assembly)
        .ok_or_else(|| anyhow!("missing assembly {assembly}"))?
        .class(name)
        .ok_or_else(|| anyhow!("missing class {name}"))
}

fn code(method: Option<Method>, name: &str) -> Result<usize> {
    let method = method.ok_or_else(|| anyhow!("missing method {name}"))?;
    if method.function.is_null() {
        bail!("{name} has no compiled code");
    }
    Ok(method.function as usize)
}

fn chunk(address: usize) -> Option<(usize, usize, bool)> {
    let mut image_base = 0u64;
    let entry = unsafe { RtlLookupFunctionEntry(address as u64, &mut image_base, None) };
    if entry.is_null() {
        return None;
    }
    let base = image_base as usize;
    let entry = unsafe { &*entry };
    let unwind_flags = unsafe { *((base + entry.Anonymous.UnwindData as usize) as *const u8) } >> 3;
    Some((base + entry.BeginAddress as usize, base + entry.EndAddress as usize, unwind_flags & UNW_FLAG_CHAININFO != 0))
}

fn body(function: usize) -> Result<&'static [u8]> {
    let (begin, mut end, _) = chunk(function).ok_or_else(|| anyhow!("no unwind info for function at {function:#x}"))?;
    while let Some((next_begin, next_end, true)) = chunk(end) {
        if next_begin != end {
            break;
        }
        end = next_end;
    }
    Ok(unsafe { std::slice::from_raw_parts(begin as *const u8, end - begin) })
}

fn find_unique(body: &[u8], aob: &str, name: &str) -> Result<usize> {
    let pattern = aobscan::PatternBuilder::from_ida_style(aob)?.build();
    let mut hits = Vec::new();
    pattern.scan(body, |offset| {
        hits.push(offset);
        true
    });
    match hits.as_slice() {
        [offset] => Ok(body.as_ptr() as usize + offset),
        [] => bail!("{name}: pattern not found (already patched or different game version)"),
        _ => bail!("{name}: pattern matched {} times", hits.len()),
    }
}

fn write(address: usize, bytes: &[u8]) -> Result<()> {
    unsafe {
        patch_bytes(address, bytes.len(), bytes)?;
        FlushInstructionCache(GetCurrentProcess(), Some(address as *const c_void), bytes.len())?;
    }
    Ok(())
}

pub fn granny_skip_animation() -> Result<()> {
    let _attached = Thread::attach(true);

    let capsule = class("Assembly-CSharp", "ColorToyCapsule")?;
    let insert_coin = code(capsule.method(("AttemptInsertCoin", 0)), "ColorToyCapsule::AttemptInsertCoin")?;
    let timer = capsule
        .nested_types
        .iter()
        .filter_map(|&ptr| cache::class_from_ptr(ptr))
        .find(|nested| nested.name.starts_with("<TimerToyCapsule>"))
        .ok_or_else(|| anyhow!("missing ColorToyCapsule.<TimerToyCapsule> coroutine"))?;
    let timer_move_next = code(timer.method(("MoveNext", 0)), "<TimerToyCapsule>::MoveNext")?;

    let play_site = find_unique(body(insert_coin)?, PLAY_AOB, "Animation.Play call")?;
    let wait_site = find_unique(body(timer_move_next)?, WAIT_AOB, "capsule timer wait")?;

    let call = play_site + PLAY_CALL;
    let rel = unsafe { (call as *const u8).add(1).cast::<i32>().read_unaligned() };
    let call_target = (call + 5).wrapping_add_signed(rel as isize);
    let animation = class("UnityEngine.AnimationModule", "UnityEngine.Animation")?;
    if !animation.methods.iter().any(|m| m.name == "Play" && m.function as usize == call_target) {
        bail!("call at {call:#x} doesn't go to Animation.Play");
    }

    for (offset, len) in PLAY_NOPS {
        write(play_site + offset, &vec![0x90; len])?;
    }
    write(wait_site, &WAIT_ZERO)?;
    Ok(())
}
