#!/usr/bin/env bash
set -euo pipefail
scratch=${CHRONO_NATIVE_SCRATCH:?missing owned native check directory}
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

coverage_failure() {
  echo "E_VERUS_COVERAGE: $*" >&2
  exit 1
}

# Cargo exits zero when a registered consumer has been removed or ignored.
consumer=atomic::late_extra_output_failure_rolls_back_nested_directories_and_prior_outputs
cargo test --color never --locked --target-dir "$scratch/tests" --manifest-path "$tests" -- --format pretty | tee "$scratch/tests.log"
grep -Fxq "test $consumer ... ok" "$scratch/tests.log" ||
  coverage_failure "the required transaction rollback consumer did not run successfully"
grep -Fxq "test raw_event_reader_checks_every_captured_identity ... ok" "$scratch/tests.log" ||
  coverage_failure "the GitHub identity consumer did not run successfully"

grep -Fxq "test live_rule_selection_follows_upstream_and_preserves_local_work ... ok" "$scratch/tests.log" &&
  grep -Fxq "test root_guides_follow_published_selection_without_losing_host_text ... ok" "$scratch/tests.log" ||
  coverage_failure "the inherited rule consumers did not exercise selection and guide sync"

grep -Fxq "test registered_guide_and_catalog_failures_preserve_the_entire_host ... ok" "$scratch/tests.log" &&
  grep -Fxq "test catalog_admission_preserves_unique_names_and_first_errors ... ok" "$scratch/tests.log" ||
  coverage_failure "the catalog admission and real guide consumer did not run successfully"

grep -Fxq "test whole_log_events_roll_only_after_the_last_available_byte ... ok" "$scratch/tests.log" &&
  grep -Fxq "test opted_in_instruction_logs_rotate_whole_events_inside_their_own_directory ... ok" "$scratch/tests.log" ||
  coverage_failure "the log write proof expectations and CLI rollover did not both pass"

# Read the exact library selected by Cargo in the build of the tested executable.
metadata=$(jq -ers --arg manifest "$(pwd -P)/$product" '
  [.[] | select(.reason == "compiler-artifact" and .manifest_path == $manifest
                 and .target.kind == ["lib"]) | .filenames[] | select(endswith(".rmeta"))]
  | if length == 1 then .[0] else error("expected the product library") end
' "$scratch/build.jsonl") || coverage_failure "native library was not identified"
filename=${metadata##*/}
stem=${filename#lib}
stem=${metadata%/*}/${stem%.rmeta}
chrono-host-bootstrap check-compiler-sources native "$stem" src/publication_decision.rs src/transaction.rs src/event_identity.rs src/github_event.rs src/rule_selection.rs src/verified_bytes.rs src/inherit.rs src/catalog_admission.rs src/catalog.rs ||
  coverage_failure "native library differs from required sources"
