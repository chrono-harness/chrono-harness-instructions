//! Identity relationships among the byte slices captured from a GitHub work event.
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

pub enum Identity {
    Valid,
    ClosingMismatch,
    ReusedId,
}

fn same_id(left: &[u8], right: &[u8]) -> (same: bool)
    ensures same == (left@ == right@),
{
    if left.len() != right.len() { return false; }
    let mut i = 0;
    while i < left.len()
        invariant i <= left.len(), left.len() == right.len(),
            forall|j: int| 0 <= j < i ==> left@[j] == right@[j],
        decreases left.len() - i,
    {
        if left[i] != right[i] { return false; }
        i += 1;
    }
    assert(left@ =~= right@);
    true
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
        if !same_id(event, end) { return Identity::ClosingMismatch; }
    }
    if same_id(session, parent) || same_id(session, task) || same_id(session, event)
        || same_id(parent, task) || same_id(parent, event) || same_id(task, event) {
        return Identity::ReusedId;
    }
    Identity::Valid
}

}
