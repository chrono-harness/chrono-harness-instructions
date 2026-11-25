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

verus! {
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
