//! Integration test against a real HEIC file.
//! Set HEIC_TEST_FILE to a .heic path to enable; skipped otherwise so CI stays green.

use std::path::Path;

#[test]
fn converts_a_real_heic() {
    let Ok(input) = std::env::var("HEIC_TEST_FILE") else {
        eprintln!("HEIC_TEST_FILE not set - skipping");
        return;
    };
    let out_dir = std::env::temp_dir().join("heic_convert_test");
    std::fs::create_dir_all(&out_dir).unwrap();

    for format in ["jpeg", "png"] {
        let out = heic_convert::convert::convert_one(
            &input,
            format,
            85,
            Some(out_dir.to_str().unwrap()),
        )
        .unwrap_or_else(|e| panic!("{format} conversion failed: {e}"));
        let meta = std::fs::metadata(Path::new(&out)).unwrap();
        assert!(meta.len() > 10_000, "{format} output suspiciously small: {} bytes", meta.len());
        println!("{format}: {} ({} KB)", out, meta.len() / 1024);
    }
}
