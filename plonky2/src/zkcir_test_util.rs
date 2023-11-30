use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use lazy_static::lazy_static;
use spin::{Mutex, MutexGuard};
use zkcir::ast::Expression;
use zkcir::ir::CirBuilder;

use crate::iop::target::Target;

lazy_static! {
    static ref TEST_PROJECTS_ROOT: PathBuf =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("cir_test_snapshots");

    // Very hacky solution to get the cir output that was last run.
    static ref LAST_CIR_DATA: Mutex<CirData> = Mutex::new(CirData {
        cir: CirBuilder::new(),
    });
}

pub struct CirData {
    pub cir: CirBuilder,
}

pub fn set_last_cir_data(cir_data: CirData) {
    let mut data = LAST_CIR_DATA.lock();
    *data = cir_data;
}

pub fn get_last_cir_data() -> MutexGuard<'static, CirData> {
    LAST_CIR_DATA.lock()
}

pub fn test_ir_string(test_name: &str, cir: &CirBuilder) {
    let cir_json = cir.to_string_omit_random().expect("couldn't serialize cir");

    let test_path = TEST_PROJECTS_ROOT.join(test_name).with_extension("json");

    if let Ok(expected) = fs::read_to_string(&test_path) {
        pretty_assertions::assert_str_eq!(
            // Must normalize newline characters otherwise testing on windows locally passes but fails
            // in github actions environment
            &expected.replace("\r\n", "\n"),
            &cir_json.replace("\r\n", "\n")
        );
    } else {
        let mut output_file = fs::File::create(test_path).expect("couldn't create output file");
        output_file
            .write_all(cir_json.as_bytes())
            .expect("couldn't write to output file.");
    }
}

pub fn target_to_ast(target: Target) -> Expression {
    match target {
        Target::Wire(w) => zkcir::ast::Expression::Wire(zkcir::ast::Wire::new(w.row, w.column)),
        Target::VirtualTarget { index } => {
            zkcir::ast::Expression::VirtualWire(zkcir::ast::VirtualWire::new(index))
        }
    }
}
