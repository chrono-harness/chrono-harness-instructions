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
cargo build --locked --target-dir "$scratch/product" --manifest-path "$product"
export CHRONO_TEST_INSTRUCTIONS="$scratch/product/debug/chrono-instructions"
cargo test --locked --target-dir "$scratch/tests" --manifest-path "$tests"

prove() {
  local label=$1 root=$2
  local output="$scratch/$label.rlib"
  "${VERUS:-verus}" --crate-type=lib --no-cheating --no-external-by-default --num-threads 2 \
    --output-json --compile "--emit=link=$output,dep-info=$output.d" "$root" -o "$output" \
    -Zchecksum-hash-algorithm=sha256 > "$scratch/proof.json" || { cat "$scratch/proof.json" >&2; return 1; }
  cat "$scratch/proof.json"
  jq -e '.verus.version == "0.2026.04.24.f8e1704" and
    .verus.commit == "f8e17045c4ac17a02e566b8997070f2eecc255be" and
    (."verification-results" | .success == true and ."is-verifying-entire-crate" == true and
    ."encountered-error" == false and ."encountered-vir-error" == false and
    .errors == 0 and .verified > 0)' "$scratch/proof.json" >/dev/null
  test -s "$output" && test -s "$output.d"
}

prove verify.instructions-publication crates/instructions/src/publication_decision.rs
prove verify.instructions-publication-tests crates/instructions-tests/tests/publication_decision.rs
