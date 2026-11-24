//! Identity relationships among the byte slices captured from a GitHub work event.
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
#[path = "verified_bytes.rs"]
mod verified_bytes;
#[cfg(not(verus_keep_ghost))]
use crate::verified_bytes;

verus! {

pub enum Identity {
    Valid,
    ClosingMismatch,
    ReusedId,
}

/// A v3 closing ID repeats the opening event. The four identity roles must be independent.
pub fn identity(session: &[u8], parent: &[u8], task: &[u8], event: &[u8], closing: Option<&[u8]>)
    -> (verdict: Identity)
    ensures
        (verdict is ClosingMismatch) == (match closing { Some(end) => event@ != end@, None => false }),
        (verdict is ReusedId) == (!(verdict is ClosingMismatch) &&
            (session@ == parent@ || session@ == task@ || session@ == event@ ||
             parent@ == task@ || parent@ == event@ || task@ == event@)),
        (verdict is Valid) == (!(verdict is ClosingMismatch) && !(verdict is ReusedId)),
{
    if let Some(end) = closing {
        if !verified_bytes::equal(event, end) { return Identity::ClosingMismatch; }
    }
    if verified_bytes::equal(session, parent) || verified_bytes::equal(session, task) || verified_bytes::equal(session, event)
        || verified_bytes::equal(parent, task) || verified_bytes::equal(parent, event) || verified_bytes::equal(task, event) {
        return Identity::ReusedId;
    }
    Identity::Valid
}

}
