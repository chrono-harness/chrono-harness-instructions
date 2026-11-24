//! Byte equality used by the work event and inherited rule decisions.
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub(crate) fn equal(left: &[u8], right: &[u8]) -> (same: bool)
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

}
