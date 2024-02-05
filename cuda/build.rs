use std::{env};

fn main() {

    // Detect if there is CUDA compiler and engage "cuda" feature accordingly
    let nvcc = match env::var("NVCC") {
        Ok(var) => which::which(var),
        Err(_) => which::which("nvcc"),
    };

    if nvcc.is_ok() {
        let mut nvcc = cc::Build::new();
        nvcc.cuda(true);
        nvcc.flag("-g");
        nvcc.flag("-O5");
        nvcc.flag("-arch=sm_75");
        nvcc.flag("-maxrregcount=255");
        nvcc.file("plonky2_gpu.cu").compile("plonky2_cuda");

        println!("cargo:rustc-cfg=feature=\"cuda\"");
        println!("cargo:rerun-if-changed=cuda");
        println!("cargo:rerun-if-env-changed=CXXFLAGS");
    } else {
        println!("nvcc must be in the path. Consider adding /usr/local/cuda/bin.");
    }
}
