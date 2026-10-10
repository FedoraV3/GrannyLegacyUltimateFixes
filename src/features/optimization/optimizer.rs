use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use il2cpp_bridge_rs::api::{self, Thread, cache};
use il2cpp_bridge_rs::structs::components::UnityObject;
use il2cpp_bridge_rs::structs::{Il2cppString, Method};
use retour::RawDetour;

type Ptr = *mut c_void;
type Str = *mut Il2cppString;

const CORE_MODULE: &str = "UnityEngine.CoreModule";
const CSHARP: &str = "Assembly-CSharp";
const AI_TAG_TTL: Duration = Duration::from_millis(250);
const MAX_CACHE_ENTRIES: usize = 4096;

static ORIG_GO_FIND: AtomicUsize = AtomicUsize::new(0);
static ORIG_SHADER_FIND: AtomicUsize = AtomicUsize::new(0);
static ORIG_FIND_WITH_TAG: AtomicUsize = AtomicUsize::new(0);
static ORIG_GRANNY_FIXED: AtomicUsize = AtomicUsize::new(0);
static ORIG_GRANDPA_FIXED: AtomicUsize = AtomicUsize::new(0);
static ORIG_SET_INT: AtomicUsize = AtomicUsize::new(0);
static ORIG_SET_FLOAT: AtomicUsize = AtomicUsize::new(0);
static ORIG_SET_STRING: AtomicUsize = AtomicUsize::new(0);
static ORIG_DELETE_KEY: AtomicUsize = AtomicUsize::new(0);
static ORIG_DELETE_ALL: AtomicUsize = AtomicUsize::new(0);
static ORIG_SAVE: AtomicUsize = AtomicUsize::new(0);

static ACTIVE_IN_HIERARCHY: AtomicUsize = AtomicUsize::new(0);
static GET_INT: AtomicUsize = AtomicUsize::new(0);
static GET_FLOAT: AtomicUsize = AtomicUsize::new(0);
static GET_STRING: AtomicUsize = AtomicUsize::new(0);
static HAS_KEY: AtomicUsize = AtomicUsize::new(0);

static PREFS_DIRTY: AtomicBool = AtomicBool::new(true);

type StrToObj = unsafe extern "C" fn(Str, Ptr) -> Ptr;
type Method0 = unsafe extern "C" fn(Ptr, Ptr);
type Static0 = unsafe extern "C" fn(Ptr);
type ObjToBool = unsafe extern "C" fn(Ptr, Ptr) -> bool;
type StrToBool = unsafe extern "C" fn(Str, Ptr) -> bool;
type StrMethod = unsafe extern "C" fn(Str, Ptr);
type SetInt = unsafe extern "C" fn(Str, i32, Ptr);
type GetInt = unsafe extern "C" fn(Str, i32, Ptr) -> i32;
type SetFloat = unsafe extern "C" fn(Str, f32, Ptr);
type GetFloat = unsafe extern "C" fn(Str, f32, Ptr) -> f32;
type SetString = unsafe extern "C" fn(Str, Str, Ptr);
type GetString = unsafe extern "C" fn(Str, Str, Ptr) -> Str;

unsafe fn func<F: Copy>(slot: &AtomicUsize) -> F {
    let addr = slot.load(Relaxed);
    unsafe { std::mem::transmute_copy(&addr) }
}

struct GcHandle(u32);

impl GcHandle {
    fn new(obj: Ptr) -> Self {
        GcHandle(unsafe { api::gchandle_new(obj, false) })
    }

    fn target(&self) -> Ptr {
        unsafe { api::gchandle_get_target(self.0) }
    }
}

impl Drop for GcHandle {
    fn drop(&mut self) {
        unsafe { api::gchandle_free(self.0) }
    }
}

unsafe fn text(s: Str) -> Option<String> {
    if s.is_null() { None } else { unsafe { (*s).to_string() } }
}

fn alive(obj: Ptr) -> bool {
    !obj.is_null() && !UnityObject::from_ptr(obj).m_cached_ptr.is_null()
}

thread_local! {
    static FIND_MAP: RefCell<HashMap<String, GcHandle>> = RefCell::new(HashMap::new());
    static SHADER_MAP: RefCell<HashMap<String, GcHandle>> = RefCell::new(HashMap::new());
    static TAG_MAP: RefCell<HashMap<String, (GcHandle, Instant)>> = RefCell::new(HashMap::new());
    static AI_DEPTH: Cell<u32> = const { Cell::new(0) };
}

fn store<V>(map: &RefCell<HashMap<String, V>>, key: String, value: Option<V>) {
    let mut map = map.borrow_mut();
    match value {
        Some(value) => {
            if map.len() >= MAX_CACHE_ENTRIES && !map.contains_key(&key) {
                map.clear();
            }
            map.insert(key, value);
        }
        None => {
            map.remove(&key);
        }
    }
}

unsafe extern "C" fn hk_gameobject_find(name: Str, method: Ptr) -> Ptr {
    let orig: StrToObj = unsafe { func(&ORIG_GO_FIND) };
    let Some(key) = (unsafe { text(name) }) else {
        return unsafe { orig(name, method) };
    };
    let cached = FIND_MAP.with(|m| m.borrow().get(&key).map(GcHandle::target));
    if let Some(obj) = cached {
        let active: ObjToBool = unsafe { func(&ACTIVE_IN_HIERARCHY) };
        if alive(obj) && unsafe { active(obj, std::ptr::null_mut()) } {
            return obj;
        }
    }
    let obj = unsafe { orig(name, method) };
    FIND_MAP.with(|m| store(m, key, (!obj.is_null()).then(|| GcHandle::new(obj))));
    obj
}

unsafe extern "C" fn hk_shader_find(name: Str, method: Ptr) -> Ptr {
    let orig: StrToObj = unsafe { func(&ORIG_SHADER_FIND) };
    let Some(key) = (unsafe { text(name) }) else {
        return unsafe { orig(name, method) };
    };
    let cached = SHADER_MAP.with(|m| m.borrow().get(&key).map(GcHandle::target));
    if let Some(shader) = cached.filter(|s| alive(*s)) {
        return shader;
    }
    let shader = unsafe { orig(name, method) };
    SHADER_MAP.with(|m| store(m, key, (!shader.is_null()).then(|| GcHandle::new(shader))));
    shader
}

unsafe extern "C" fn hk_find_gameobjects_with_tag(tag: Str, method: Ptr) -> Ptr {
    let orig: StrToObj = unsafe { func(&ORIG_FIND_WITH_TAG) };
    if AI_DEPTH.get() == 0 {
        return unsafe { orig(tag, method) };
    }
    let Some(key) = (unsafe { text(tag) }) else {
        return unsafe { orig(tag, method) };
    };
    let cached = TAG_MAP.with(|m| {
        m.borrow()
            .get(&key)
            .filter(|(_, at)| at.elapsed() < AI_TAG_TTL)
            .map(|(h, _)| h.target())
    });
    if let Some(array) = cached.filter(|a| !a.is_null()) {
        return array;
    }
    let array = unsafe { orig(tag, method) };
    TAG_MAP.with(|m| store(m, key, (!array.is_null()).then(|| (GcHandle::new(array), Instant::now()))));
    array
}

unsafe fn in_ai(slot: &AtomicUsize, this: Ptr, method: Ptr) {
    let orig: Method0 = unsafe { func(slot) };
    AI_DEPTH.set(AI_DEPTH.get() + 1);
    unsafe { orig(this, method) };
    AI_DEPTH.set(AI_DEPTH.get() - 1);
}

unsafe extern "C" fn hk_granny_fixed_update(this: Ptr, method: Ptr) {
    unsafe { in_ai(&ORIG_GRANNY_FIXED, this, method) }
}

unsafe extern "C" fn hk_grandpa_fixed_update(this: Ptr, method: Ptr) {
    unsafe { in_ai(&ORIG_GRANDPA_FIXED, this, method) }
}

unsafe extern "C" fn hk_set_int(key: Str, value: i32, method: Ptr) {
    if !key.is_null() {
        let get: GetInt = unsafe { func(&GET_INT) };
        if unsafe { get(key, value ^ 1, std::ptr::null_mut()) } == value {
            return;
        }
    }
    let orig: SetInt = unsafe { func(&ORIG_SET_INT) };
    unsafe { orig(key, value, method) };
    PREFS_DIRTY.store(true, Relaxed);
}

unsafe extern "C" fn hk_set_float(key: Str, value: f32, method: Ptr) {
    if !key.is_null() {
        let get: GetFloat = unsafe { func(&GET_FLOAT) };
        let fallback = if value == 1.0 { 2.0 } else { 1.0 };
        if unsafe { get(key, fallback, std::ptr::null_mut()) }.to_bits() == value.to_bits() {
            return;
        }
    }
    let orig: SetFloat = unsafe { func(&ORIG_SET_FLOAT) };
    unsafe { orig(key, value, method) };
    PREFS_DIRTY.store(true, Relaxed);
}

unsafe extern "C" fn hk_set_string(key: Str, value: Str, method: Ptr) {
    if !key.is_null() && !value.is_null() {
        let has: StrToBool = unsafe { func(&HAS_KEY) };
        let get: GetString = unsafe { func(&GET_STRING) };
        if unsafe { has(key, std::ptr::null_mut()) } {
            let current = unsafe { text(get(key, value, std::ptr::null_mut())) };
            if current.is_some() && current == unsafe { text(value) } {
                return;
            }
        }
    }
    let orig: SetString = unsafe { func(&ORIG_SET_STRING) };
    unsafe { orig(key, value, method) };
    PREFS_DIRTY.store(true, Relaxed);
}

unsafe extern "C" fn hk_delete_key(key: Str, method: Ptr) {
    let orig: StrMethod = unsafe { func(&ORIG_DELETE_KEY) };
    unsafe { orig(key, method) };
    PREFS_DIRTY.store(true, Relaxed);
}

unsafe extern "C" fn hk_delete_all(method: Ptr) {
    let orig: Static0 = unsafe { func(&ORIG_DELETE_ALL) };
    unsafe { orig(method) };
    PREFS_DIRTY.store(true, Relaxed);
}

unsafe extern "C" fn hk_save(method: Ptr) {
    if !PREFS_DIRTY.load(Relaxed) {
        return;
    }
    PREFS_DIRTY.store(false, Relaxed);
    let orig: Static0 = unsafe { func(&ORIG_SAVE) };
    unsafe { orig(method) };
}

fn resolve(assembly: &str, class: &str, method: &str, argc: usize) -> Result<Method> {
    let info = cache::assembly(assembly)
        .ok_or_else(|| anyhow!("missing assembly {assembly}"))?
        .class(class)
        .ok_or_else(|| anyhow!("missing class {class}"))?
        .method((method, argc))
        .ok_or_else(|| anyhow!("missing method {class}::{method}/{argc}"))?;
    if info.function.is_null() {
        return Err(anyhow!("{class}::{method}/{argc} has no compiled code"));
    }
    Ok(info)
}

struct Hook {
    assembly: &'static str,
    class: &'static str,
    method: &'static str,
    argc: usize,
    detour: *const (),
    orig: &'static AtomicUsize,
    optional: bool,
}

pub fn install_optimizations() -> Result<()> {
    let _attached = Thread::attach(true);

    for (class, method, argc, slot) in [
        ("UnityEngine.GameObject", "get_activeInHierarchy", 0, &ACTIVE_IN_HIERARCHY),
        ("UnityEngine.PlayerPrefs", "GetInt", 2, &GET_INT),
        ("UnityEngine.PlayerPrefs", "GetFloat", 2, &GET_FLOAT),
        ("UnityEngine.PlayerPrefs", "GetString", 2, &GET_STRING),
        ("UnityEngine.PlayerPrefs", "HasKey", 1, &HAS_KEY),
    ] {
        slot.store(resolve(CORE_MODULE, class, method, argc)?.function as usize, Relaxed);
    }

    let hook = |assembly, class, method, argc, detour: *const (), orig| Hook { assembly, class, method, argc, detour, orig, optional: false };
    let optional = |h: Hook| Hook { optional: true, ..h };
    let hooks = [
        hook(CORE_MODULE, "UnityEngine.GameObject", "Find", 1, hk_gameobject_find as *const (), &ORIG_GO_FIND),
        hook(CORE_MODULE, "UnityEngine.Shader", "Find", 1, hk_shader_find as *const (), &ORIG_SHADER_FIND),
        hook(CORE_MODULE, "UnityEngine.GameObject", "FindGameObjectsWithTag", 1, hk_find_gameobjects_with_tag as *const (), &ORIG_FIND_WITH_TAG),
        hook(CSHARP, "AI_Granny", "FixedUpdate", 0, hk_granny_fixed_update as *const (), &ORIG_GRANNY_FIXED),
        hook(CSHARP, "AI_Grandpa", "FixedUpdate", 0, hk_grandpa_fixed_update as *const (), &ORIG_GRANDPA_FIXED),
        hook(CORE_MODULE, "UnityEngine.PlayerPrefs", "SetInt", 2, hk_set_int as *const (), &ORIG_SET_INT),
        hook(CORE_MODULE, "UnityEngine.PlayerPrefs", "SetFloat", 2, hk_set_float as *const (), &ORIG_SET_FLOAT),
        hook(CORE_MODULE, "UnityEngine.PlayerPrefs", "SetString", 2, hk_set_string as *const (), &ORIG_SET_STRING),
        optional(hook(CORE_MODULE, "UnityEngine.PlayerPrefs", "DeleteKey", 1, hk_delete_key as *const (), &ORIG_DELETE_KEY)),
        optional(hook(CORE_MODULE, "UnityEngine.PlayerPrefs", "DeleteAll", 0, hk_delete_all as *const (), &ORIG_DELETE_ALL)),
        hook(CORE_MODULE, "UnityEngine.PlayerPrefs", "Save", 0, hk_save as *const (), &ORIG_SAVE),
    ];

    let mut targets = Vec::with_capacity(hooks.len());
    for h in &hooks {
        let target = match resolve(h.assembly, h.class, h.method, h.argc) {
            Ok(info) => info.function as usize,
            Err(_) if h.optional => continue,
            Err(e) => return Err(e),
        };
        if targets.iter().any(|(_, t)| *t == target) {
            return Err(anyhow!("{}::{} shares code with another hooked method", h.class, h.method));
        }
        targets.push((h, target));
    }

    for (h, target) in targets {
        let detour = unsafe { RawDetour::new(target as *const (), h.detour) }
            .map_err(|e| anyhow!("detour {}::{}: {e}", h.class, h.method))?;
        h.orig.store(detour.trampoline() as *const () as usize, Relaxed);
        unsafe { detour.enable() }.map_err(|e| anyhow!("enable {}::{}: {e}", h.class, h.method))?;
        Box::leak(Box::new(detour));
    }
    Ok(())
}
