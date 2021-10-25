#!/bin/sh

rm -rf tmp/pgo

if !(rustup help >/dev/null); then
	echo "Rustup not found; ensure rustup is on PATH" 1>&2
	exit 1
fi

if !(rustup component add llvm-tools-preview); then
	echo "Could not install llvm-tools-preview" 1>&2
	exit 1
fi

TOOLCHAIN_BASE=$(dirname $(dirname $(rustup which cargo)))
TOOLCHAIN_NAME=$(basename $TOOLCHAIN_BASE)
TARGET_TRIPLE=${TOOLCHAIN_NAME#*"-"} # e.g. nightly-x86_64-apple-darwin -> x86_64-apple-darwin
PROFDATA_PATH=$TOOLCHAIN_BASE/lib/rustlib/$TARGET_TRIPLE/bin/llvm-profdata

if !(RUSTFLAGS="$RUSTFLAGS -Cprofile-generate=$(pwd)/tmp/pgo/data -Ctarget-cpu=native" cargo test --target-dir tmp/pgo/target --release test_recursive_recursive_verifier -- --ignored); then
	echo "Build failed" 1>&2
	exit 1
fi
if !($PROFDATA_PATH merge -o pgo-data.profdata tmp/pgo/data); then
	echo "Could not create .profdata file" 1>&2
	exit 1
fi

rm -rf tmp/pgo

echo '.profdata file successfully created. Add "-Cprofile-use=$(pwd)/pgo-data.profdata" to RUSTFLAGS to use it.' 1>&2
