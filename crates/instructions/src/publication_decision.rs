//! Decide which exact outputs may be restored after an in-flight generation failure.
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
