// The engine as WebAssembly, for the server and the page (site/js/engine.js wraps this). Text goes in
// and out through the module's memory:
//   alloc(n) -> where to write n bytes;  free(where, n) when done with them
//   out() -> where the last answer is (its length is what the call returned)
//   island(where, n) -> the answer's length. The input is "<seed>\n<settings text>" (settings.rs);
//     the answer is the island as JSON (island.rs to_json), or "error <why>"
//   settings_table() -> the answer's length: every setting as JSON (Settings::table_json)
//   source_hash() -> the answer's length: the source's fingerprint (build.rs)
use crate::island::generate;
use crate::settings::Settings;
use std::cell::RefCell;

thread_local! {
    static OUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

fn answer(text: String) -> usize {
    OUT.with(|o| {
        let mut o = o.borrow_mut();
        *o = text.into_bytes();
        o.len()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn alloc(n: usize) -> *mut u8 {
    let mut v = Vec::<u8>::with_capacity(n.max(1));
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

/// # Safety
/// `p` and `n` must be what alloc() gave and was asked for.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn free(p: *mut u8, n: usize) {
    unsafe { drop(Vec::from_raw_parts(p, 0, n.max(1))) }
}

#[unsafe(no_mangle)]
pub extern "C" fn out() -> *const u8 {
    OUT.with(|o| o.borrow().as_ptr())
}

/// # Safety
/// `p` must point to `n` bytes of UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn island(p: *const u8, n: usize) -> usize {
    let input = unsafe { std::slice::from_raw_parts(p, n) };
    let Ok(text) = std::str::from_utf8(input) else { return answer("error the input isn't UTF-8".into()) };
    let (seed, settings) = text.split_once('\n').unwrap_or((text, ""));
    answer(match Settings::parse(settings).and_then(|s| generate(&s, seed)) {
        Ok(island) => island.to_json(),
        Err(why) => format!("error {why}"),
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn settings_table() -> usize {
    answer(Settings::table_json())
}

#[unsafe(no_mangle)]
pub extern "C" fn source_hash() -> usize {
    answer(crate::SOURCE_HASH.into())
}
