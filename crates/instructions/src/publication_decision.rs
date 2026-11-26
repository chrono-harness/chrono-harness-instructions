//! Decide which exact outputs may be restored after an in-flight generation failure.
#[cfg(verus_keep_ghost)]
#[path = "../examples/publish_rules.rs"]
pub(crate) mod publish_rules;
#[cfg(verus_keep_ghost)]
#[path = "transaction.rs"]
pub(crate) mod transaction;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

// The test build observes the actual invocation on the current thread. The
// released tool does not install an observer or retain any state for this.
#[cfg(all(feature = "test-support", not(verus_keep_ghost)))]
pub(crate) mod observation {
    use std::cell::Cell;

    thread_local! {
        static CALLED: Cell<bool> = const { Cell::new(false) };
    }

    pub(super) fn mark() {
        CALLED.set(true);
    }

    pub(crate) fn observe<T>(operation: impl FnOnce() -> T) -> (T, bool) {
        CALLED.set(false);
        let value = operation();
        (value, CALLED.get())
    }
}

verus! {

/// Preserve the original logging error only when the command streams were delivered.
pub fn finish_cli<E>(streams_written: bool, log_error: Option<E>, business: u8)
    -> (result: (u8, Option<E>))
    ensures
        result.0 == if !streams_written { 1u8 } else if log_error.is_some() { 2u8 } else { business },
        result.1 == if streams_written { log_error } else { None },
{
    if !streams_written { return (1, None); }
    match log_error {
        Some(error) => (2, Some(error)),
        None => (business, None),
    }
}

pub fn event_fits(bytes: u64, limit: u64) -> (fits: bool)
    ensures fits == (bytes <= limit),
{
    bytes <= limit
}

/// Append the admitted event or rotate under the held log-directory lock.
pub enum LogWrite {
    TooLarge,
    Append,
    Rotate { archived_files: usize },
}

pub fn log_write(length: u64, bytes: u64, limit: u64, max_files: usize) -> (action: LogWrite)
    ensures match action {
        LogWrite::TooLarge => bytes > limit || max_files == 0,
        LogWrite::Append => bytes <= limit && max_files > 0 && length + bytes <= limit,
        LogWrite::Rotate { archived_files } => bytes <= limit && max_files > 0
            && length + bytes > limit && archived_files + 1 == max_files,
    },
{
    if bytes > limit || max_files == 0 { return LogWrite::TooLarge; }
    if length > limit - bytes {
        LogWrite::Rotate { archived_files: max_files - 1 }
    } else {
        LogWrite::Append
    }
}

pub enum Repair {
    Leave,
    Restore,
    Preserve,
}

pub fn repair(before: bool, installed: bool, backup_ready: bool, committed: bool) -> (action: Repair)
    ensures
        (action is Leave) == (if committed { installed } else { before }),
        (action is Restore) == (!committed && !before && installed && backup_ready),
        (action is Preserve) == (!(if committed { installed } else { before })
                                 && !(!committed && !before && installed && backup_ready)),
{
    #[cfg(all(feature = "test-support", not(verus_keep_ghost)))]
    observation::mark();
    if if committed { installed } else { before } {
        Repair::Leave
    } else if !committed && installed && backup_ready {
        Repair::Restore
    } else {
        Repair::Preserve
    }
}

}
