//! 与 IO 实现独立的恢复、释放边界预期；同源 Verus 和 Rust 均执行。
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../instructions/src/publication_decision.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_instructions::publication_decision as policy;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

#[cfg(not(verus_keep_ghost))]
#[allow(dead_code)]
#[path = "../../instructions/examples/publish_rules.rs"]
mod publish_rules;
#[cfg(verus_keep_ghost)]
use policy::publish_rules;

#[path = "../../../.chrono-harness/ci/roadmap.rs"]
mod roadmap;

verus! {
#[cfg(verus_keep_ghost)]
fn commit_observation_matches_only_the_complete_original_identity(
    identity: &[u8], other: &[u8], digest: &[u8; 32], changed: &[u8; 32],
) {
    use policy::transaction::same_observation;
    let expected = (identity, 0o644u32, Some(digest));
    let exact = same_observation(expected, expected);
    let mode_drift = same_observation((identity, 0o600, Some(digest)), expected);
    let no_bytes = same_observation((identity, 0o644, None), expected);
    let identity_check = same_observation((other, 0o644, Some(digest)), expected);
    let contents_check = same_observation((identity, 0o644, Some(changed)), expected);
    assert(exact && !mode_drift && !no_bytes);
    assert(identity_check == (identity@ == other@));
    assert(contents_check == (digest@ == changed@));
}

#[cfg(verus_keep_ghost)]
fn hosted_publication_follows_observed_refs_and_diff(current: bool, existing: bool, code: i32) {
    let base = publish_rules::publication_base(current, existing);
    if !current { assert(base is Stale); }
    else if existing { assert(base is Existing); }
    else { assert(base is First); }
    let change = publish_rules::staged_change(code);
    if code == 0 { assert(change == Some(false)); }
    else if code == 1 { assert(change == Some(true)); }
    else { assert(change == None::<bool>); }
}

#[cfg(verus_keep_ghost)]
fn changed_digest_at_any_byte_rejects(before: &[u8; 32], after: &[u8; 32], at: usize)
    requires at < 32, before@[at as int] != after@[at as int],
{
    let accepted = policy::transaction::same_digest(Some(before), Some(after));
    assert(!accepted);
}

#[cfg(verus_keep_ghost)]
fn equal_and_absent_digests(bytes: &[u8; 32]) {
    let identical = policy::transaction::same_digest(Some(bytes), Some(bytes));
    assert(identical);
    let both_absent = policy::transaction::same_digest(None, None);
    assert(both_absent);
    let missing_right = policy::transaction::same_digest(Some(bytes), None);
    assert(!missing_right);
    let missing_left = policy::transaction::same_digest(None, Some(bytes));
    assert(!missing_left);
}

// The deployed shell passes the required names to the same checker exercised below.
pub fn native_recipe() -> &'static str {
    include_str!("../../../.chrono-harness/ci/native.sh")
}

#[cfg(verus_keep_ghost)]
fn exact_native_result(lines: &[&[u8]], expected: &[u8], at: usize)
    requires at < lines.len(), lines@[at as int]@ == expected@,
{
    let present = policy::required_consumers::has_line(lines, expected);
    assert(present);
}

#[cfg(verus_keep_ghost)]
fn absent_native_result(lines: &[&[u8]], expected: &[u8])
    requires forall|j: int| 0 <= j < lines.len() ==> lines@[j]@ != expected@,
{
    let present = policy::required_consumers::has_line(lines, expected);
    assert(!present);
}
fn expect_true(actual: bool)
    requires actual,
{
    if !actual { panic!(); }
}

fn failing_output_takes_priority_over_every_cli_or_log_result(business: u8, error: Option<u64>) {
    let (exit, message) = policy::finish_cli(false, error, business);
    assert(exit == 1 && message.is_none());
}

fn written_output_preserves_the_business_exit_or_original_logging_error(business: u8, id: u64) {
    let (ok, untouched) = policy::finish_cli::<u64>(true, None, business);
    assert(ok == business && untouched.is_none());
    let (failed, original) = policy::finish_cli(true, Some(id), business);
    assert(failed == 2 && original == Some(id));
}

#[cfg(verus_keep_ghost)]
fn completed_diagnostics_session_preserves_business_or_reports_logging_failure(business: u8, other: i32) {
    let ok = policy::runtime_log::session_exit(business, Some(business as i32));
    assert(ok == Some(business));
    let failed = policy::runtime_log::session_exit(business, Some(2));
    assert(failed == Some(2u8));
    let signal = policy::runtime_log::session_exit(business, None);
    assert(signal.is_none());
    let unexpected = policy::runtime_log::session_exit(business, Some(other));
    if other != business as i32 && other != 2 { assert(unexpected.is_none()); }
}

#[cfg(verus_keep_ghost)]
fn command_stream_failure_has_precedence_and_success_retains_the_real_outcome(business: u8, status: u8, error: u64) {
    let transport = policy::cli_entry::stream_result(true, Err(error), business);
    assert(transport == (2u8, Some(error)));
    let lost = policy::cli_entry::stream_result(false, Err(error), business);
    assert(lost == (1u8, None::<u64>));
    let exact = policy::cli_entry::stream_result::<u64>(true, Ok(status), business);
    assert(exact == (status,None::<u64>));
    let write = policy::cli_entry::stream_result::<u64>(false, Ok(status), business);
    assert(write == (1u8,None::<u64>));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn rollback_only_replaces_the_exact_installed_object() {
    let untouched = policy::repair(true, false, false, false);
    let fresh = policy::repair(false, true, true, false);
    let lost_original = policy::repair(false, true, false, false);
    let drift = policy::repair(false, false, true, false);
    let missing = policy::repair(false, false, false, false);
    expect_true(matches!(untouched, policy::Repair::Leave));
    expect_true(matches!(fresh, policy::Repair::Restore));
    expect_true(matches!(lost_original, policy::Repair::Preserve));
    expect_true(matches!(drift, policy::Repair::Preserve));
    expect_true(matches!(missing, policy::Repair::Preserve));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn immutable_commit_never_rolls_back_and_requires_installed_identity() {
    let installed = policy::repair(false, true, true, true);
    let original = policy::repair(true, false, true, true);
    let missing = policy::repair(false, false, false, true);
    expect_true(matches!(installed, policy::Repair::Leave));
    expect_true(matches!(original, policy::Repair::Preserve));
    expect_true(matches!(missing, policy::Repair::Preserve));
}

#[cfg(not(verus_keep_ghost))]
#[test]
#[should_panic]
fn expectations_have_a_real_failure_branch() { expect_true(false); }
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn checked_rule_publication_does_not_push_stale_unchanged_or_failed_data() {
    use publish_rules::{GitResult, publish};
    // Each invocation supplies the actual command boundary. The test never runs Git.
    for (dev, existing, diff, stages, pushes, ok) in [
        ("newer", true, 1, false, false, true),
        ("checked", false, 1, true, true, true),
        ("checked", true, 1, true, true, true),
        ("checked", true, 0, true, false, true),
        ("checked", true, 128, true, false, false),
    ] {
        let mut calls = Vec::new();
        let mut staged = false;
        let result = publish(
            "checked",
            |args| {
                calls.push(args.to_vec().join(" "));
                let (code, stdout) = match args {
                    ["ls-remote", "origin", "refs/heads/dev"] => {
                        (0, format!("{dev}\trefs/heads/dev\n"))
                    }
                    [
                        "ls-remote",
                        "--heads",
                        "origin",
                        "refs/heads/published-rules",
                    ] => (
                        0,
                        if existing {
                            "base\trefs/heads/published-rules\n"
                        } else {
                            ""
                        }
                        .into(),
                    ),
                    ["diff", "--cached", "--quiet"] => (diff, String::new()),
                    _ => (0, String::new()),
                };
                Ok(GitResult { code, stdout })
            },
            || {
                staged = true;
                Ok(())
            },
        );
        assert_eq!(result.is_ok(), ok);
        assert_eq!(staged, stages);
        let mut expected = vec!["ls-remote origin refs/heads/dev"];
        if stages {
            expected.push("ls-remote --heads origin refs/heads/published-rules");
            if existing {
                expected.extend([
                    "fetch --no-tags --depth=1 origin refs/heads/published-rules",
                    "switch --detach FETCH_HEAD",
                ]);
            } else {
                expected.push("switch --orphan published-rules");
            }
            expected.extend([
                "add -- rules.json methodology-attribution.md trureturing-Apache-2.0.txt",
                "diff --cached --quiet",
            ]);
            if pushes {
                expected.extend(["-c user.name=github-actions -c user.email=41898282+github-actions[bot]@users.noreply.github.com commit -m Publish rules from checked", "push origin HEAD:refs/heads/published-rules"]);
            }
        }
        assert_eq!(calls, expected);
    }
}

#[cfg(not(verus_keep_ghost))]
#[allow(dead_code)]
#[path = "../../instructions/examples/required_consumers.rs"]
mod required_consumers;

#[cfg(not(verus_keep_ghost))]
#[test]
fn native_recipe_requires_actual_success_for_each_registered_consumer() {
    let script = native_recipe();
    let call = script
        .lines()
        .position(|line| {
            line.starts_with("cargo run ") && line.contains("--example required_consumers -- ")
        })
        .unwrap();
    let lines: Vec<_> = script.lines().collect();
    assert!(lines[call - 1].starts_with("cargo test "));
    assert!(lines[call - 1].ends_with("tee \"$scratch/tests.log\""));
    assert!(lines[call].ends_with("\"$scratch/tests.log\" \\"));
    let names: Vec<_> = lines[call + 1..]
        .iter()
        .take_while(|line| line.starts_with("  "))
        .map(|line| line.trim().trim_end_matches(" \\"))
        .collect();
    assert_eq!(
        names,
        [
            "atomic::late_extra_output_failure_rolls_back_nested_directories_and_prior_outputs",
            "raw_event_reader_checks_every_captured_identity",
            "live_rule_selection_follows_upstream_and_preserves_local_work",
            "root_guides_follow_published_selection_without_losing_host_text",
            "registered_guide_and_catalog_failures_preserve_the_entire_host",
            "catalog_admission_preserves_unique_names_and_first_errors",
            "cli_runtime_uses_the_selected_host_and_public_diagnostics_command",
            "public_log_session_preserves_command_streams_and_joins_every_started_process",
        ]
    );
    let receipts: Vec<_> = names
        .iter()
        .map(|name| format!("test {name} ... ok"))
        .collect();
    for (i, row) in receipts.iter().enumerate() {
        let lines: Vec<_> = receipts.iter().map(|s| s.as_bytes()).collect();
        assert!(required_consumers::has_line(&lines, row.as_bytes()));
        let mut missing = lines;
        missing[i] = b"test other ... ok";
        assert!(!required_consumers::has_line(&missing, row.as_bytes()));
    }
}
