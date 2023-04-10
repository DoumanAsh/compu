use core::ffi::CStr;

#[inline(always)]
///This always rely on the fact that most common libraries return static strings.
///If that's not the case, do not use this function
pub fn convert_c_str(ptr: *const i8) -> Option<&'static str> {
    if ptr.is_null() {
        return None
    } else {
        let text = unsafe {
            CStr::from_ptr(ptr)
        };
        text.to_str().ok()
    }
}
