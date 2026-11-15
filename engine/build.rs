// A fingerprint of the engine's source, built into it (SOURCE_HASH), so tests/engine.test.mjs can
// tell whether site/js/settlers.wasm was built from the source as it is now. FNV-1a (64-bit) over
// every .rs file directly in src/, sorted by name, with carriage returns left out (Windows and Linux
// checkouts differ). src/bin/ isn't included: the command-line tool isn't part of the .wasm.
fn main() {
    println!("cargo:rerun-if-changed=src");
    let mut files: Vec<_> = std::fs::read_dir("src")
        .expect("can't read src/")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "rs"))
        .collect();
    files.sort_by_key(|p| p.file_name().unwrap().to_string_lossy().into_owned());
    let mut h: u64 = 0xcbf29ce484222325;
    for f in &files {
        let bytes = std::fs::read(f).unwrap_or_else(|e| panic!("can't read {}: {e}", f.display()));
        for &b in bytes.iter().filter(|&&b| b != b'\r') {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
    }
    println!("cargo:rustc-env=SOURCE_HASH={h:016x}");
}
