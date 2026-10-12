#[allow(unused)]
pub(crate) fn ue_random_range_int(min_inclusive: i64, max_inclusive: i64, _method_info: *const std::ffi::c_void) -> i64 {
    // cannot fail
    rand::random_range(min_inclusive..=max_inclusive)
}