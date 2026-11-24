//! Select actual upstream rows using the host's decoded policy and map lookup evidence.
#[cfg(not(verus_keep_ghost))]
use crate::verified_bytes;
#[cfg(verus_keep_ghost)]
#[path = "verified_bytes.rs"]
mod verified_bytes;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

pub enum Mode { All, Only, Off }
pub enum Row { Publish, Omit, Invalid }

pub fn needs_feed(mode: &Mode, names: usize) -> (needed: bool)
    ensures needed == (mode is All || (mode is Only && names > 0)),
{
    match mode { Mode::All => true, Mode::Only => names > 0, Mode::Off => false }
}

pub fn schema(actual: &[u8], expected: &[u8]) -> (valid: bool)
    ensures valid == (actual@ == expected@),
{ verified_bytes::equal(actual, expected) }

pub open spec fn matches_id(actual: Option<&[u8]>, id: &[u8]) -> bool {
    match actual { Some(value) => value@ == id@, None => false }
}

fn lookup_matches(actual: Option<&[u8]>, id: &[u8]) -> (yes: bool)
    ensures yes == matches_id(actual, id),
{
    match actual { Some(value) => verified_bytes::equal(value, id), None => false }
}

/// selected and previous are the values returned by BTreeSet for this exact ID.
pub fn row(mode: &Mode, id: &[u8], trimmed: &[u8], selected: Option<&[u8]>, previous: Option<&[u8]>)
    -> (result: Row)
    ensures (result is Invalid) == (id.len() == 0 || trimmed.len() == 0 || matches_id(previous, id)),
        (result is Publish) == (!(result is Invalid) && match mode {
            Mode::All => !matches_id(selected, id), Mode::Only => matches_id(selected, id), Mode::Off => false,
        }),
        (result is Omit) == (!(result is Invalid) && !(result is Publish)),
{
    if id.len() == 0 || trimmed.len() == 0 || lookup_matches(previous, id) { return Row::Invalid; }
    let listed = lookup_matches(selected, id);
    match mode {
        Mode::All if !listed => Row::Publish,
        Mode::Only if listed => Row::Publish,
        _ => Row::Omit,
    }
}

pub open spec fn prefix_at(text: Seq<u8>, marker: Seq<u8>, start: int) -> bool {
    0 <= start && start + marker.len() <= text.len() &&
    forall|k:int| 0 <= k < marker.len() ==> #[trigger] text[start + k] == marker[k]
}

/// Host-owned guide delimiters must not be introduced anywhere in the selected projection.
pub fn safe_projection(text: &[u8], marker: &[u8]) -> (safe: bool)
    ensures safe == !(exists|i:int| #[trigger] prefix_at(text@,marker@,i)),
{
    if marker.len() > text.len() { return true; }
    let last = text.len() - marker.len();
    let mut start = 0;
    loop
        invariant start <= last, last == text.len() - marker.len(), marker.len() <= text.len(),
            forall|i:int| 0 <= i < start ==> !prefix_at(text@,marker@,i),
        decreases last - start,
    {
        let mut k = 0;
        while k < marker.len() && text[start+k] == marker[k]
            invariant k <= marker.len(), start + marker.len() <= text.len(),
                forall|j:int| 0 <= j < k ==> #[trigger] text@[start as int + j] == marker@[j],
            decreases marker.len() - k,
        { k += 1; }
        if k == marker.len() {
            assert(prefix_at(text@,marker@,start as int));
            return false;
        }
        assert(!prefix_at(text@,marker@,start as int)) by {
            if prefix_at(text@,marker@,start as int) { assert(text@[start as int + k] == marker@[k as int]); }
        }
        if start == last { return true; }
        start += 1;
    }
}

}
