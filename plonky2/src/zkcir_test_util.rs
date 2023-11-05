use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use lazy_static::lazy_static;
use zkcir::ir::CirBuilder;

lazy_static! {
    // Very hacky solution to get the cir that was last run. This assumes that tests are run once at a time.
    static ref CIR: Mutex<CirBuilder> = Mutex::new(CirBuilder::new());

    static ref TEST_PROJECTS_ROOT: PathBuf =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("cir_test_snapshots");
}

pub fn set_cir(new_cir: CirBuilder) {
    let mut data = CIR.lock().unwrap();
    *data = new_cir;
}

pub fn get_last_cir() -> CirBuilder {
    *CIR.lock().unwrap()
}

pub fn test_ir_string(test_name: &str, cir: String) {
    let test_path = TEST_PROJECTS_ROOT.join(test_name).with_extension("json");

    if let Ok(expected) = fs::read_to_string(&test_path) {
        pretty_assertions::assert_str_eq!(
            // Must normalize newline characters otherwise testing on windows locally passes but fails
            // in github actions environment
            &expected.replace("\r\n", "\n"),
            &cir.replace("\r\n", "\n")
        );
    } else {
        let mut output_file = fs::File::create(test_path).expect("couldn't create output file");
        output_file
            .write_all(cir.as_bytes())
            .expect("couldn't write fixed.diff to output file.");
    }
}
