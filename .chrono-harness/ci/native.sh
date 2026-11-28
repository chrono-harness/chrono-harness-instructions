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

coverage_failure() {
  echo "E_VERUS_COVERAGE: $*" >&2
  exit 1
}

# Cargo exits zero when a registered consumer has been removed or ignored.
cargo test --color never --locked --target-dir "$scratch/tests" --manifest-path "$tests" -- --format pretty | tee "$scratch/tests.log"
cargo run --quiet --locked --manifest-path "$product" --target-dir "$scratch/product" --example required_consumers -- "$scratch/tests.log" \
  atomic::late_extra_output_failure_rolls_back_nested_directories_and_prior_outputs \
  atomic::graph_integrity_includes_unselected_atoms_and_diagnoses_chains \
  atomic::layout_rejects_invalid_placement_locale_and_depth_before_any_writes \
  atomic::invalid_skill_metadata_and_format_bindings_fail_without_writes \
  input_drift_before_commit_rolls_back_without_touching_the_external_edit \
  raw_event_reader_checks_every_captured_identity \
  live_rule_selection_follows_upstream_and_preserves_local_work \
  root_guides_follow_published_selection_without_losing_host_text \
  registered_guide_and_catalog_failures_preserve_the_entire_host \
  catalog_admission_preserves_unique_names_and_first_errors \
  cli_runtime_uses_the_selected_host_and_public_diagnostics_command \
  public_log_session_preserves_command_streams_and_joins_every_started_process

# Read the exact library selected by Cargo in the build of the tested executable.
stem=$(chrono-host-bootstrap select-native-cargo-library "$(pwd -P)/$product" "$scratch/build.jsonl") ||
  coverage_failure "native library was not identified"
chrono-host-bootstrap check-compiler-sources native "$stem" src/publication_decision.rs src/transaction.rs src/event_identity.rs src/github_event.rs src/rule_selection.rs src/verified_bytes.rs src/inherit.rs src/catalog_admission.rs src/catalog.rs ||
  coverage_failure "native library differs from required sources"
