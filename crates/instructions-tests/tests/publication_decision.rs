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

verus! {
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

#[cfg_attr(not(verus_keep_ghost), test)]
fn whole_log_events_roll_only_after_the_last_available_byte() {
    use policy::{event_fits, log_write, LogWrite};
    expect_true(event_fits(128, 128));
    expect_true(!event_fits(129, 128));
    expect_true(matches!(log_write(123, 5, 128, 2), LogWrite::Append));
    expect_true(matches!(log_write(124, 5, 128, 2), LogWrite::Rotate { archived_files: 1 }));
    expect_true(matches!(log_write(128, 1, 128, 1), LogWrite::Rotate { archived_files: 0 }));
    expect_true(matches!(log_write(0, 129, 128, 2), LogWrite::TooLarge));
    expect_true(matches!(log_write(u64::MAX, 1, 128, 2), LogWrite::Rotate { archived_files: 1 }));
    expect_true(matches!(log_write(0, 1, 128, 0), LogWrite::TooLarge));
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
