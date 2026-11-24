#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p target
scratch=$(mktemp -d "$(pwd -P)/target/check.XXXXXXXX")
trap 'rm -rf -- "$scratch"' EXIT
export TMPDIR="$scratch"
product=crates/instructions/Cargo.toml
tests=crates/instructions-tests/Cargo.toml
for manifest in "$product" "$tests"; do
  cargo metadata --locked --all-features --manifest-path "$manifest" --format-version 1 > "$scratch/dependencies.json"
  jq -e --arg product "$(pwd -P)/$product" --arg manifest "$(pwd -P)/$manifest" '
    all(.packages[];
      if .source == null then (.manifest_path == $product or .manifest_path == $manifest)
      elif (.source | startswith("registry+")) then true
      elif (.source | startswith("git+")) then
        (.source | test("github\\.com[:/]((chrono-harness/)|(ChronoAIProject/chrono-harness([./?#]|$)))"; "i") | not)
      else false end)
  ' "$scratch/dependencies.json" >/dev/null || { echo "unexpected dependency source in $manifest" >&2; exit 1; }
  cargo fmt --check --manifest-path "$manifest"
done
export CARGO_NET_OFFLINE=true
cargo build --locked --message-format=json-render-diagnostics --target-dir "$scratch/product" --manifest-path "$product" > "$scratch/build.jsonl"
export CHRONO_TEST_INSTRUCTIONS="$scratch/product/debug/chrono-instructions"
cargo test --locked --target-dir "$scratch/tests" --manifest-path "$tests"

coverage_failure() {
  echo "E_VERUS_COVERAGE: $*" >&2
  exit 1
}

prove() {
  local label=$1 root=$2
  local output="$scratch/$label.rlib" report="$scratch/$label.json"
  "${VERUS:-verus}" --crate-type=lib --no-cheating --no-external-by-default --num-threads 2 \
    --output-json --compile "--emit=link=$output,dep-info=$output.d" "$root" -o "$output" \
    -Zchecksum-hash-algorithm=sha256 > "$report" || { cat "$report" >&2; coverage_failure "$label: verifier failed"; }
}

admit_proof() {
  local label=$1
  shift
  chrono-host-bootstrap check-compiler-proof \
    0.2026.04.24.f8e1704 f8e17045c4ac17a02e566b8997070f2eecc255be \
    "$scratch/$label.json" "$scratch/$label.rlib" "$@" ||
    coverage_failure "$label: required proof result was not admitted"
}

decision=crates/instructions/src/publication_decision.rs
expectations=crates/instructions-tests/tests/publication_decision.rs
# Read the exact library selected by Cargo in the build of the tested executable.
metadata=$(jq -ers --arg manifest "$(pwd -P)/$product" '
  [.[] | select(.reason == "compiler-artifact" and .manifest_path == $manifest
                 and .target.kind == ["lib"]) | .filenames[] | select(endswith(".rmeta"))]
  | if length == 1 then .[0] else error("expected the product library") end
' "$scratch/build.jsonl") || coverage_failure "native library was not identified"
filename=${metadata##*/}
stem=${filename#lib}
stem=${metadata%/*}/${stem%.rmeta}
chrono-host-bootstrap check-compiler-sources native "$stem" src/publication_decision.rs src/transaction.rs ||
  coverage_failure "native library differs from required sources"


product_operation=verify.instructions-publication
test_operation=verify.instructions-publication-tests

prove "$product_operation" "$decision"
prove "$test_operation" "$expectations"

admit_proof "$product_operation" "$decision"
admit_proof "$test_operation" "$expectations" crates/instructions-tests/tests/../../instructions/src/publication_decision.rs
