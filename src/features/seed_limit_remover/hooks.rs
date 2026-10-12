use std::ffi::{CString, c_void};
use std::mem::transmute_copy;
use std::ptr::null_mut;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

use anyhow::{Context, Result, bail};
use il2cpp_bridge_rs::api::{self, cache};
use il2cpp_bridge_rs::structs::Il2cppString;
use windows::Win32::System::LibraryLoader::GetModuleHandleA;
use windows::core::s;

use crate::memory::memory_constants::generics as rva;

type Ptr = *mut c_void;

const CSHARP: &str = "Assembly-CSharp";
const CORE_MODULE: &str = "UnityEngine.CoreModule";

const MAX_ATTEMPTS: i32 = 50;
const RANDOM_SEED_EXCLUSIVE_MAX: i64 = 999_999_999;
const CATEGORY_EXCLUDED: i32 = 4;

mod layout {
    pub const ALL_ITEMS: usize = 0x20;
    pub const PUZZLE_DEFS: usize = 0x28;
    pub const REQUIRED_ESCAPE_ITEM_NAMES: usize = 0x30;
    pub const SPAWN_AREAS: usize = 0x38;
    pub const SEED: usize = 0x40;
    pub const RANDOMIZE_SEED: usize = 0x44;
    pub const FILL_ALL_FREE_SPAWNS: usize = 0x46;
    pub const ESCAPE_ITEM_PUZZLE_CHANCE: usize = 0x48;
    pub const RND: usize = 0x50;
    pub const ITEMS_BY_NAME: usize = 0x58;
    pub const USED_ITEM_NAMES: usize = 0x60;
    pub const USED_PUZZLE_SPAWNS: usize = 0x68;
    pub const PUZZLE_HAS_ITEM: usize = 0x70;

    pub const PUZZLE_NAME: usize = 0x10;
    pub const PUZZLE_SPAWN_POINT: usize = 0x18;
    pub const PUZZLE_PRIORITY: usize = 0x30;

    pub const ITEM_NAME: usize = 0x20;
    pub const ITEM_CATEGORY: usize = 0x28;

    pub const AREA_CURRENT_ITEMS: usize = 0x24;
    pub const AREA_USED_SPOTS: usize = 0x28;
    pub const AREA_MANDATORY_ITEM_NAME: usize = 0x38;

    pub const CACHED_PTR: usize = 0x10;
    pub const LIST_ITEMS: usize = 0x10;
    pub const LIST_SIZE: usize = 0x18;
    pub const ARRAY_LENGTH: usize = 0x18;
    pub const ARRAY_DATA: usize = 0x20;
    pub const STRING_LENGTH: usize = 0x10;

    pub const MENU_IS_TYPING: usize = 0x20;
    pub const MENU_SEED_INPUT: usize = 0x28;
    pub const MENU_SEED_PREF_KEY: usize = 0x38;
    pub const INPUT_TEXT: usize = 0x220;
}
use layout::*;

type Method0 = unsafe extern "C" fn(Ptr, Ptr);
type Method0Bool = unsafe extern "C" fn(Ptr, Ptr) -> bool;
type PlaceInFreeArea = unsafe extern "C" fn(Ptr, Ptr, Ptr);
type PlaceAtTransform = unsafe extern "C" fn(Ptr, Ptr, Ptr, bool, Ptr);
type PlaceInSpecificArea = unsafe extern "C" fn(Ptr, Ptr, Ptr, Ptr);
type ItemPuzzleCheck = unsafe extern "C" fn(Ptr, Ptr, Ptr, Ptr) -> bool;
type PrefsGetInt = unsafe extern "C" fn(Ptr, Ptr) -> i32;
type RandomCtor = unsafe extern "C" fn(Ptr, i32, Ptr);
type RandomNext = unsafe extern "C" fn(Ptr, i32, Ptr) -> i32;
type RandomNextDouble = unsafe extern "C" fn(Ptr, Ptr) -> f64;
type Clear = unsafe extern "C" fn(Ptr, Ptr);
type SetCall = unsafe extern "C" fn(Ptr, Ptr, Ptr) -> bool;
type DictionarySet = unsafe extern "C" fn(Ptr, Ptr, Ptr, Ptr);
type DictionaryGet = unsafe extern "C" fn(Ptr, Ptr, Ptr, Ptr);
type DictionaryTryGet = unsafe extern "C" fn(Ptr, Ptr, Ptr, Ptr) -> bool;
type GetComponent = unsafe extern "C" fn(Ptr, Ptr, Ptr);
type Shuffle = unsafe extern "C" fn(Ptr, Ptr, Ptr);
type InitMetadataSlot = unsafe extern "C" fn(*mut Ptr);

struct SetMethods {
    clear: Ptr,
    add: Ptr,
    contains: Ptr,
}

struct Api {
    save_original_states: Method0,
    restore_original_states: Method0,
    build_lookup: Method0,
    place_in_free_area: PlaceInFreeArea,
    place_at_transform: PlaceAtTransform,
    place_in_specific_area: PlaceInSpecificArea,
    detect_circular_dependencies: Method0Bool,
    validate_puzzle_dependencies: Method0Bool,
    is_safe_container_placement: ItemPuzzleCheck,
    is_item_allowed_for_puzzle: ItemPuzzleCheck,
    prefs_get_int: PrefsGetInt,
    random_class: Ptr,
    random_ctor: RandomCtor,
    random_next: RandomNext,
    random_next_double: RandomNextDouble,
    set_clear: Clear,
    set_add: SetCall,
    set_contains: SetCall,
    dictionary_clear: Clear,
    dictionary_set: DictionarySet,
    dictionary_get: DictionaryGet,
    dictionary_try_get: DictionaryTryGet,
    list_clear: Clear,
    get_component: GetComponent,
    shuffle: Shuffle,
    names: SetMethods,
    spawns: SetMethods,
    mi_has_item_clear: Ptr,
    mi_has_item_set: Ptr,
    mi_has_item_get: Ptr,
    mi_items_by_name_try_get: Ptr,
    mi_list_transform_clear: Ptr,
    mi_get_item_seed_data: Ptr,
    mi_shuffle_game_object: Ptr,
}

unsafe impl Send for Api {}
unsafe impl Sync for Api {}

static API: OnceLock<Api> = OnceLock::new();
static ORIGINAL: AtomicUsize = AtomicUsize::new(0);

unsafe fn func<F: Copy>(address: usize) -> F {
    unsafe { transmute_copy(&address) }
}

fn method_address(assembly: &str, class: &str, name: &str, argc: usize) -> Result<usize> {
    let method = cache::assembly(assembly)
        .with_context(|| format!("missing assembly {assembly}"))?
        .class(class)
        .with_context(|| format!("missing class {class}"))?
        .method((name, argc))
        .with_context(|| format!("missing method {class}::{name}/{argc}"))?;
    if method.function.is_null() {
        bail!("{class}::{name}/{argc} has no compiled code");
    }
    Ok(method.function as usize)
}

impl Api {
    unsafe fn resolve() -> Result<Self> {
        let base = unsafe { GetModuleHandleA(s!("GameAssembly.dll")) }
            .context("GameAssembly.dll is not loaded")?
            .0 as usize;

        let init_slot: InitMetadataSlot = unsafe { func(base + rva::INIT_METADATA_SLOT) };
        let slot = |offset: usize| -> Result<Ptr> {
            let slot = (base + offset) as *mut Ptr;
            unsafe { init_slot(slot) };
            let value = unsafe { slot.read() };
            if value.is_null() {
                bail!("metadata slot 0x{offset:X} did not initialise");
            }
            Ok(value)
        };

        let seed_manager = |name: &str, argc: usize| method_address(CSHARP, "SeedManager", name, argc);
        let random = cache::mscorlib().class("System.Random").context("missing class System.Random")?;
        let random_class = random.address;
        let random_method = |name: &str, argc: usize| {
            random
                .method((name, argc))
                .filter(|m| !m.function.is_null())
                .map(|m| m.function as usize)
                .with_context(|| format!("missing method System.Random::{name}/{argc}"))
        };

        Ok(Api {
            save_original_states: unsafe { func(seed_manager("SaveOriginalStates", 0)?) },
            restore_original_states: unsafe { func(seed_manager("RestoreOriginalStates", 0)?) },
            build_lookup: unsafe { func(seed_manager("BuildLookup", 0)?) },
            place_in_free_area: unsafe { func(seed_manager("PlaceItemInFreeArea", 1)?) },
            place_at_transform: unsafe { func(seed_manager("PlaceItemAtTransform", 3)?) },
            place_in_specific_area: unsafe { func(seed_manager("PlaceItemInSpecificFreeArea", 2)?) },
            detect_circular_dependencies: unsafe { func(seed_manager("DetectCircularDependencies", 0)?) },
            validate_puzzle_dependencies: unsafe { func(seed_manager("ValidatePuzzleDependencies", 0)?) },
            is_safe_container_placement: unsafe { func(seed_manager("IsSafeContainerPlacement", 2)?) },
            is_item_allowed_for_puzzle: unsafe { func(seed_manager("IsItemAllowedForPuzzle", 2)?) },

            prefs_get_int: unsafe { func(method_address(CORE_MODULE, "UnityEngine.PlayerPrefs", "GetInt", 1)?) },

            random_class,
            random_ctor: unsafe { func(random_method(".ctor", 1)?) },
            random_next: unsafe { func(random_method("Next", 1)?) },
            random_next_double: unsafe { func(random_method("NextDouble", 0)?) },

            set_clear: unsafe { func(base + rva::HASHSET_CLEAR) },
            set_add: unsafe { func(base + rva::HASHSET_ADD) },
            set_contains: unsafe { func(base + rva::HASHSET_CONTAINS) },
            dictionary_clear: unsafe { func(base + rva::DICTIONARY_CLEAR) },
            dictionary_set: unsafe { func(base + rva::DICTIONARY_SET_ITEM) },
            dictionary_get: unsafe { func(base + rva::DICTIONARY_GET_ITEM) },
            dictionary_try_get: unsafe { func(base + rva::DICTIONARY_TRY_GET_VALUE) },
            list_clear: unsafe { func(base + rva::LIST_CLEAR) },
            get_component: unsafe { func(base + rva::GET_COMPONENT) },
            shuffle: unsafe { func(base + rva::SHUFFLE) },

            names: SetMethods {
                clear: slot(rva::MI_HASHSET_STRING_CLEAR)?,
                add: slot(rva::MI_HASHSET_STRING_ADD)?,
                contains: slot(rva::MI_HASHSET_STRING_CONTAINS)?,
            },
            spawns: SetMethods {
                clear: slot(rva::MI_HASHSET_TRANSFORM_CLEAR)?,
                add: slot(rva::MI_HASHSET_TRANSFORM_ADD)?,
                contains: slot(rva::MI_HASHSET_TRANSFORM_CONTAINS)?,
            },
            mi_has_item_clear: slot(rva::MI_DICTIONARY_STRING_BOOL_CLEAR)?,
            mi_has_item_set: slot(rva::MI_DICTIONARY_STRING_BOOL_SET_ITEM)?,
            mi_has_item_get: slot(rva::MI_DICTIONARY_STRING_BOOL_GET_ITEM)?,
            mi_items_by_name_try_get: slot(rva::MI_DICTIONARY_STRING_GAMEOBJECT_TRY_GET_VALUE)?,
            mi_list_transform_clear: slot(rva::MI_LIST_TRANSFORM_CLEAR)?,
            mi_get_item_seed_data: slot(rva::MI_GET_COMPONENT_ITEM_SEED_DATA)?,
            mi_shuffle_game_object: slot(rva::MI_SHUFFLE_GAMEOBJECT)?,
        })
    }
}

pub fn init() -> Result<()> {
    if API.get().is_some() {
        return Ok(());
    }
    let api = unsafe { Api::resolve() }?;
    let _ = API.set(api);
    Ok(())
}

pub fn set_original(trampoline: usize) {
    ORIGINAL.store(trampoline, Relaxed);
}

unsafe fn read<T: Copy>(object: Ptr, offset: usize) -> T {
    unsafe { (object as *const u8).add(offset).cast::<T>().read_unaligned() }
}

unsafe fn write<T>(object: Ptr, offset: usize, value: T) {
    unsafe { (object as *mut u8).add(offset).cast::<T>().write_unaligned(value) }
}

unsafe fn alive(object: Ptr) -> bool {
    !object.is_null() && unsafe { read::<usize>(object, CACHED_PTR) } != 0
}

unsafe fn is_null_or_empty(string: Ptr) -> bool {
    string.is_null() || unsafe { read::<i32>(string, STRING_LENGTH) } == 0
}

unsafe fn list_items(list: Ptr) -> Vec<Ptr> {
    if list.is_null() {
        return Vec::new();
    }
    let (items, size) = unsafe { (read::<Ptr>(list, LIST_ITEMS), read::<i32>(list, LIST_SIZE)) };
    if items.is_null() || size <= 0 {
        return Vec::new();
    }
    let count = (size as usize).min(unsafe { read::<usize>(items, ARRAY_LENGTH) });
    (0..count).map(|i| unsafe { read::<Ptr>(items, ARRAY_DATA + i * 8) }).collect()
}

unsafe fn array_items(array: Ptr) -> Vec<Ptr> {
    if array.is_null() {
        return Vec::new();
    }
    let count = unsafe { read::<usize>(array, ARRAY_LENGTH) };
    (0..count).map(|i| unsafe { read::<Ptr>(array, ARRAY_DATA + i * 8) }).collect()
}

unsafe fn managed_list_from(template: Ptr, items: &[Ptr]) -> Option<Ptr> {
    unsafe {
        if template.is_null() {
            return None;
        }
        let backing = read::<Ptr>(template, LIST_ITEMS);
        if backing.is_null() {
            return None;
        }
        let element_class = api::class_get_element_class(read::<Ptr>(backing, 0));
        let array = api::array_new(element_class, items.len() as u32);
        for (i, item) in items.iter().enumerate() {
            write::<Ptr>(array, ARRAY_DATA + i * 8, *item);
        }
        let list = api::object_new(read::<Ptr>(template, 0));
        write::<Ptr>(list, LIST_ITEMS, array);
        write::<i32>(list, LIST_SIZE, items.len() as i32);
        Some(list)
    }
}

unsafe fn managed_string(text: &str) -> Ptr {
    let text = CString::new(text).unwrap_or_default();
    unsafe { api::string_new(text.as_ptr()) }
}

fn rng_seed(seed: i64, attempt: i32) -> i32 {
    let folded = i32::try_from(seed).unwrap_or_else(|_| (seed as i32) ^ ((seed >> 32) as i32));
    folded.wrapping_add(attempt)
}

unsafe fn pick_seed(api: &Api, this: Ptr) -> i64 {
    unsafe {
        let randomize_pref = (api.prefs_get_int)(managed_string("RandomSeed"), null_mut()) == 1;
        if randomize_pref && read::<u8>(this, RANDOMIZE_SEED) != 0 {
            rand::random_range(0..RANDOM_SEED_EXCLUSIVE_MAX)
        } else {
            stored_seed().unwrap_or_else(|| (api.prefs_get_int)(managed_string("GameSeed"), null_mut()) as i64)
        }
    }
}

struct Attempt<'a> {
    api: &'a Api,
    this: Ptr,
    rnd: Ptr,
    used_names: Ptr,
    used_spawns: Ptr,
    puzzle_has_item: Ptr,
    items_by_name: Ptr,
}

impl Attempt<'_> {
    unsafe fn next(&self, bound: usize) -> usize {
        unsafe { (self.api.random_next)(self.rnd, bound as i32, null_mut()) as usize }
    }

    unsafe fn next_double(&self) -> f64 {
        unsafe { (self.api.random_next_double)(self.rnd, null_mut()) }
    }

    unsafe fn name_used(&self, name: Ptr) -> bool {
        unsafe { (self.api.set_contains)(self.used_names, name, self.api.names.contains) }
    }

    unsafe fn mark_name_used(&self, name: Ptr) {
        unsafe { (self.api.set_add)(self.used_names, name, self.api.names.add) };
    }

    unsafe fn spawn_used(&self, spawn: Ptr) -> bool {
        unsafe { (self.api.set_contains)(self.used_spawns, spawn, self.api.spawns.contains) }
    }

    unsafe fn set_puzzle_has_item(&self, puzzle_name: Ptr, value: bool) {
        unsafe { (self.api.dictionary_set)(self.puzzle_has_item, puzzle_name, &value as *const bool as Ptr, self.api.mi_has_item_set) };
    }

    unsafe fn puzzle_has_item(&self, puzzle_name: Ptr) -> bool {
        let mut out = 0u64;
        unsafe { (self.api.dictionary_get)(self.puzzle_has_item, puzzle_name, &mut out as *mut u64 as Ptr, self.api.mi_has_item_get) };
        out as u8 != 0
    }

    unsafe fn find_item(&self, name: Ptr) -> Option<Ptr> {
        let mut item: Ptr = null_mut();
        let found = unsafe { (self.api.dictionary_try_get)(self.items_by_name, name, &mut item as *mut Ptr as Ptr, self.api.mi_items_by_name_try_get) };
        found.then_some(item)
    }

    unsafe fn item_data(&self, item: Ptr) -> Ptr {
        let mut data: Ptr = null_mut();
        if !item.is_null() {
            unsafe { (self.api.get_component)(item, &mut data as *mut Ptr as Ptr, self.api.mi_get_item_seed_data) };
        }
        data
    }

    unsafe fn has_seed_data(&self, item: Ptr) -> bool {
        unsafe { alive(item) && alive(self.item_data(item)) }
    }

    unsafe fn item_name(&self, item: Ptr) -> Ptr {
        let data = unsafe { self.item_data(item) };
        if data.is_null() { null_mut() } else { unsafe { read(data, ITEM_NAME) } }
    }

    unsafe fn item_unused(&self, item: Ptr) -> bool {
        unsafe { alive(self.item_data(item)) && !self.name_used(self.item_name(item)) }
    }

    unsafe fn item_placeable(&self, item: Ptr) -> bool {
        let data = unsafe { self.item_data(item) };
        unsafe { alive(data) && read::<i32>(data, ITEM_CATEGORY) != CATEGORY_EXCLUDED }
    }

    unsafe fn item_allowed(&self, item: Ptr, puzzle: Ptr) -> bool {
        unsafe { (self.api.is_item_allowed_for_puzzle)(self.this, item, puzzle, null_mut()) }
    }

    unsafe fn item_safe(&self, item: Ptr, puzzle: Ptr) -> bool {
        unsafe { (self.api.is_safe_container_placement)(self.this, item, puzzle, null_mut()) }
    }

    unsafe fn puzzle_open_for(&self, item: Ptr, puzzle: Ptr, reserved: Ptr) -> bool {
        unsafe {
            let spawn = read::<Ptr>(puzzle, PUZZLE_SPAWN_POINT);
            alive(spawn) && !self.spawn_used(spawn) && !alive(reserved) && self.item_allowed(item, puzzle)
        }
    }

    unsafe fn place_in_free_area(&self, item: Ptr) {
        unsafe { (self.api.place_in_free_area)(self.this, item, null_mut()) };
    }

    unsafe fn place_at_puzzle(&self, item: Ptr, puzzle: Ptr, name: Ptr) {
        unsafe {
            let spawn = read::<Ptr>(puzzle, PUZZLE_SPAWN_POINT);
            (self.api.place_at_transform)(self.this, item, spawn, false, null_mut());
            (self.api.set_add)(self.used_spawns, spawn, self.api.spawns.add);
            self.mark_name_used(name);
            self.set_puzzle_has_item(read(puzzle, PUZZLE_NAME), true);
        }
    }
}

#[allow(clippy::cognitive_complexity, clippy::cyclomatic_complexity)]
unsafe fn attempt_placement(api: &Api, this: Ptr, seed: i64, attempt: i32) -> Option<bool> {
    unsafe {
        let rnd = api::object_new(api.random_class);
        (api.random_ctor)(rnd, rng_seed(seed, attempt), null_mut());
        write::<Ptr>(this, RND, rnd);
        (api.restore_original_states)(this, null_mut());

        let used_names = read::<Ptr>(this, USED_ITEM_NAMES);
        let used_spawns = read::<Ptr>(this, USED_PUZZLE_SPAWNS);
        let puzzle_has_item = read::<Ptr>(this, PUZZLE_HAS_ITEM);
        let puzzle_defs_list = read::<Ptr>(this, PUZZLE_DEFS);
        let all_items_list = read::<Ptr>(this, ALL_ITEMS);
        let escape_names_list = read::<Ptr>(this, REQUIRED_ESCAPE_ITEM_NAMES);
        let spawn_areas = read::<Ptr>(this, SPAWN_AREAS);
        let items_by_name = read::<Ptr>(this, ITEMS_BY_NAME);
        if [used_names, used_spawns, puzzle_has_item, puzzle_defs_list, all_items_list, escape_names_list, spawn_areas, items_by_name]
            .iter()
            .any(|p| p.is_null())
        {
            return None;
        }

        let at = Attempt { api, this, rnd, used_names, used_spawns, puzzle_has_item, items_by_name };

        (api.set_clear)(used_names, api.names.clear);
        (api.set_clear)(used_spawns, api.spawns.clear);
        (api.dictionary_clear)(puzzle_has_item, api.mi_has_item_clear);

        let puzzle_defs = list_items(puzzle_defs_list);
        for &puzzle in &puzzle_defs {
            if puzzle.is_null() || read::<Ptr>(puzzle, PUZZLE_NAME).is_null() {
                return None;
            }
            at.set_puzzle_has_item(read(puzzle, PUZZLE_NAME), false);
        }

        let areas = array_items(spawn_areas);
        for &area in &areas {
            if area.is_null() || read::<Ptr>(area, AREA_USED_SPOTS).is_null() {
                return None;
            }
            write::<i32>(area, AREA_CURRENT_ITEMS, 0);
            (api.list_clear)(read(area, AREA_USED_SPOTS), api.mi_list_transform_clear);
        }

        (api.build_lookup)(this, null_mut());

        let all_items = list_items(all_items_list);
        let valid: Vec<Ptr> = all_items.iter().copied().filter(|&item| at.has_seed_data(item)).collect();

        let mut sorted: Vec<Ptr> = puzzle_defs
            .iter()
            .copied()
            .filter(|&puzzle| alive(read(puzzle, PUZZLE_SPAWN_POINT)))
            .collect();
        sorted.sort_by_key(|&puzzle| read::<i32>(puzzle, PUZZLE_PRIORITY));

        let mut candidates: Vec<Vec<Ptr>> = sorted
            .iter()
            .map(|&puzzle| {
                valid
                    .iter()
                    .copied()
                    .filter(|&item| at.item_unused(item) && at.item_allowed(item, puzzle) && at.item_placeable(item) && at.item_safe(item, puzzle))
                    .collect()
            })
            .collect();

        let mut reserved: Vec<Ptr> = vec![null_mut(); sorted.len()];
        for i in 0..sorted.len() {
            if candidates[i].is_empty() {
                continue;
            }
            let pick = candidates[i][at.next(candidates[i].len())];
            reserved[i] = pick;
            for (j, list) in candidates.iter_mut().enumerate() {
                if j != i {
                    if let Some(position) = list.iter().position(|&item| item == pick) {
                        list.remove(position);
                    }
                }
            }
        }

        let chance = read::<f32>(this, ESCAPE_ITEM_PUZZLE_CHANCE) as f64;
        for name in list_items(escape_names_list) {
            let Some(item) = at.find_item(name) else { continue };
            if !alive(item) || reserved.contains(&item) {
                continue;
            }
            let mut target = None;
            if chance > at.next_double() && !puzzle_defs.is_empty() {
                let open: Vec<Ptr> = (0..sorted.len())
                    .filter(|&i| at.puzzle_open_for(item, sorted[i], reserved[i]))
                    .map(|i| sorted[i])
                    .collect();
                if !open.is_empty() {
                    target = Some(open[at.next(open.len())]);
                }
            }
            match target {
                Some(puzzle) => at.place_at_puzzle(item, puzzle, name),
                None => {
                    at.place_in_free_area(item);
                    at.mark_name_used(name);
                }
            }
        }

        for i in 0..sorted.len() {
            let puzzle = sorted[i];
            if at.spawn_used(read(puzzle, PUZZLE_SPAWN_POINT)) || !alive(reserved[i]) {
                continue;
            }
            let item = reserved[i];
            let name = at.item_name(item);
            if at.name_used(name) {
                reserved[i] = null_mut();
            } else {
                at.place_at_puzzle(item, puzzle, name);
            }
        }

        for &puzzle in &sorted {
            if at.spawn_used(read(puzzle, PUZZLE_SPAWN_POINT)) || at.puzzle_has_item(read(puzzle, PUZZLE_NAME)) {
                continue;
            }
            let mut pool: Vec<Ptr> = valid
                .iter()
                .copied()
                .filter(|&item| at.item_unused(item) && at.item_allowed(item, puzzle) && at.item_placeable(item))
                .collect();
            if pool.is_empty() {
                pool = valid.iter().copied().filter(|&item| at.item_unused(item) && at.item_placeable(item)).collect();
            }
            if pool.is_empty() {
                continue;
            }
            let item = pool[at.next(pool.len())];
            at.place_at_puzzle(item, puzzle, at.item_name(item));
        }

        for &area in &areas {
            let name = read::<Ptr>(area, AREA_MANDATORY_ITEM_NAME);
            if is_null_or_empty(name) {
                continue;
            }
            let Some(item) = at.find_item(name) else { continue };
            if !at.name_used(name) {
                (api.place_in_specific_area)(this, item, area, null_mut());
                at.mark_name_used(name);
            }
        }

        if read::<u8>(this, FILL_ALL_FREE_SPAWNS) != 0 {
            let leftover: Vec<Ptr> = all_items
                .iter()
                .copied()
                .filter(|&item| !item.is_null() && alive(at.item_data(item)) && at.item_unused(item))
                .collect();
            let list = managed_list_from(all_items_list, &leftover)?;
            (api.shuffle)(this, list, api.mi_shuffle_game_object);
            for item in list_items(list) {
                at.place_in_free_area(item);
            }
        }

        Some(!(api.detect_circular_dependencies)(this, null_mut()) && (api.validate_puzzle_dependencies)(this, null_mut()))
    }
}

unsafe fn generate(api: &Api, this: Ptr) -> Option<()> {
    unsafe {
        (api.save_original_states)(this, null_mut());

        let seed = pick_seed(api, this);
        write::<i32>(this, SEED, seed as i32);

        let mut attempt = 0;
        let mut finished = false;
        while attempt < MAX_ATTEMPTS && !finished {
            attempt += 1;
            finished = attempt_placement(api, this, seed, attempt)?;
        }
    }
    Some(())
}

pub unsafe extern "C" fn generate_placement(this: Ptr, method: Ptr) {
    let done = API.get().is_some_and(|api| unsafe { generate(api, this) }.is_some());
    if done {
        return;
    }
    let original = ORIGINAL.load(Relaxed);
    if original != 0 {
        let original: Method0 = unsafe { func(original) };
        unsafe { original(this, method) };
    }
}

const SEED_PREF: &str = "GameSeed";
const SEED64_PREF: &str = "GameSeed64";

type PrefsGetString = unsafe extern "C" fn(Ptr, Ptr, Ptr) -> Ptr;
type PrefsSetString = unsafe extern "C" fn(Ptr, Ptr, Ptr);
type PrefsSetInt = unsafe extern "C" fn(Ptr, i32, Ptr);
type PrefsSave = unsafe extern "C" fn(Ptr);

struct MenuApi {
    get_int: PrefsGetInt,
    get_string: PrefsGetString,
    set_string: PrefsSetString,
    set_int: PrefsSetInt,
    save: PrefsSave,
}

unsafe impl Send for MenuApi {}
unsafe impl Sync for MenuApi {}

static MENU: OnceLock<MenuApi> = OnceLock::new();
static ORIGINAL_MENU_START: AtomicUsize = AtomicUsize::new(0);
static ORIGINAL_MENU_UPDATE: AtomicUsize = AtomicUsize::new(0);
static ORIGINAL_MENU_EXIT_TYPING: AtomicUsize = AtomicUsize::new(0);

pub fn init_menu() -> Result<()> {
    if MENU.get().is_some() {
        return Ok(());
    }
    let prefs = |name: &str, argc: usize| method_address(CORE_MODULE, "UnityEngine.PlayerPrefs", name, argc);
    let menu = unsafe {
        MenuApi {
            get_int: func(prefs("GetInt", 1)?),
            get_string: func(prefs("GetString", 2)?),
            set_string: func(prefs("SetString", 2)?),
            set_int: func(prefs("SetInt", 2)?),
            save: func(prefs("Save", 0)?),
        }
    };
    let _ = MENU.set(menu);
    Ok(())
}

pub fn set_original_menu_start(trampoline: usize) {
    ORIGINAL_MENU_START.store(trampoline, Relaxed);
}

pub fn set_original_menu_update(trampoline: usize) {
    ORIGINAL_MENU_UPDATE.store(trampoline, Relaxed);
}

pub fn set_original_menu_exit_typing(trampoline: usize) {
    ORIGINAL_MENU_EXIT_TYPING.store(trampoline, Relaxed);
}

unsafe fn managed_text(string: Ptr) -> Option<String> {
    if string.is_null() { None } else { unsafe { (*(string as *mut Il2cppString)).to_string() } }
}

fn parse_seed(typed: &str) -> i64 {
    let typed = typed.trim();
    if let Ok(seed) = typed.parse::<i64>() {
        return seed;
    }
    let digits = typed.strip_prefix('-').unwrap_or(typed);
    if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
        return if typed.starts_with('-') { i64::MIN } else { i64::MAX };
    }
    0
}

pub(crate) fn stored_seed() -> Option<i64> {
    let menu = MENU.get()?;
    unsafe {
        let wide = managed_text((menu.get_string)(managed_string(SEED64_PREF), managed_string(""), null_mut()))?
            .trim()
            .parse::<i64>()
            .ok()?;
        let narrow = (menu.get_int)(managed_string(SEED_PREF), null_mut());
        (wide as i32 == narrow).then_some(wide)
    }
}

unsafe fn save_typed_seed(menu: &MenuApi, this: Ptr) {
    unsafe {
        let input = read::<Ptr>(this, MENU_SEED_INPUT);
        let key = read::<Ptr>(this, MENU_SEED_PREF_KEY);
        if input.is_null() || key.is_null() {
            return;
        }
        let seed = parse_seed(&managed_text(read(input, INPUT_TEXT)).unwrap_or_default());
        (menu.set_int)(key, seed as i32, null_mut());
        (menu.set_string)(managed_string(SEED64_PREF), managed_string(&seed.to_string()), null_mut());
        (menu.save)(null_mut());
    }
}

unsafe fn call_original(slot: &AtomicUsize, this: Ptr, method: Ptr) {
    let original = slot.load(Relaxed);
    if original != 0 {
        let original: Method0 = unsafe { func(original) };
        unsafe { original(this, method) };
    }
}

pub unsafe extern "C" fn menu_seed_start(this: Ptr, method: Ptr) {
    unsafe { call_original(&ORIGINAL_MENU_START, this, method) };
    let Some(seed) = stored_seed().filter(|seed| i32::try_from(*seed).is_err()) else { return };
    unsafe {
        let input = read::<Ptr>(this, MENU_SEED_INPUT);
        if input.is_null() {
            return;
        }
        let set_text = cache::class_from_ptr(read::<Ptr>(input, 0))
            .and_then(|class| class.method(("set_text", 1)))
            .filter(|method| !method.function.is_null());
        if let Some(set_text) = set_text {
            let set_text: PrefsSetString = func(set_text.function as usize);
            set_text(input, managed_string(&seed.to_string()), null_mut());
        }
    }
}

pub unsafe extern "C" fn menu_seed_update(this: Ptr, method: Ptr) {
    let was_typing = unsafe { read::<u8>(this, MENU_IS_TYPING) } != 0;
    unsafe { call_original(&ORIGINAL_MENU_UPDATE, this, method) };
    if was_typing && unsafe { read::<u8>(this, MENU_IS_TYPING) } == 0 {
        if let Some(menu) = MENU.get() {
            unsafe { save_typed_seed(menu, this) };
        }
    }
}

pub unsafe extern "C" fn menu_seed_exit_typing(this: Ptr, method: Ptr) {
    unsafe { call_original(&ORIGINAL_MENU_EXIT_TYPING, this, method) };
    if let Some(menu) = MENU.get() {
        unsafe { save_typed_seed(menu, this) };
    }
}
