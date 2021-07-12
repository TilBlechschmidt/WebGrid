#!/bin/sh
OUTPUT_FORMAT="${1:-html}"
PROFILE_DIR=./target/debug/coverage

rm -rf $PROFILE_DIR
mkdir -p $PROFILE_DIR

RUSTFLAGS="-Zinstrument-coverage -Clink-dead-code" \
	LLVM_PROFILE_FILE="$PROFILE_DIR/coverage-pid%p.profraw" \
	cargo +nightly test -- --include-ignored

echo "Generating coverage with format $OUTPUT_FORMAT"

grcov $PROFILE_DIR \
	--binary-path ./target/debug/ \
	-s . \
	-t $OUTPUT_FORMAT \
	--llvm \
	--branch \
	-o $PROFILE_DIR/report \
	--ignore 'src/bin/**/*.rs' \
	--ignore build.rs \
	--keep-only 'src/**/*.rs' \
	--excl-line '#\[derive'

cargo +nightly udeps
