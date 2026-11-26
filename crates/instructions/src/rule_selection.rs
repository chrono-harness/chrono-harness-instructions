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

#[cfg(verus_keep_ghost)]
#[path = "catalog.rs"]
pub mod catalog;

#[path = "catalog_admission.rs"]
pub mod catalog_admission;

#[cfg(all(feature = "test-support", not(verus_keep_ghost)))]
pub fn rule_order(catalog: &[u8], output: &[u8]) -> Result<Vec<String>, String> {
    let catalog: crate::catalog::Catalog = crate::catalog::parse(catalog)?;
    let output = crate::catalog::parse(output)?;
    catalog
        .ordered_rule_ids(&output)
        .map(|ids| ids.into_iter().map(str::to_owned).collect())
}

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

/// The replacement range includes both markers and leaves the closing line ending with the host.
pub enum GuideEdit { Append(bool), Replace(usize, usize), Malformed, InvalidLine }

/// Publish the primary guide's exact bytes only when the second rendered root agrees.
pub fn consolidate_guides(primary: Vec<u8>, other: Option<&[u8]>) -> (result: Result<Vec<u8>, ()>)
    ensures
        result.is_ok() == match other { None => true, Some(candidate) => candidate@ == primary@ },
        result.is_ok() ==> result.unwrap()@ == primary@,
{
    if let Some(candidate) = other {
        if !verified_bytes::equal(primary.as_slice(), candidate) {
            return Err(());
        }
    }
    Ok(primary)
}

pub open spec fn standalone(bytes: Seq<u8>, start: int, width: nat) -> bool {
    let end = start + width;
    0 <= start <= end <= bytes.len()
        && (start == 0 || bytes[start - 1] == 10)
        && (end == bytes.len() || bytes[end] == 10 ||
            (end + 1 < bytes.len() && bytes[end] == 13 && bytes[end + 1] == 10))
}

fn marker_line(bytes: &[u8], start: usize, width: usize) -> (ok: bool)
    ensures ok == standalone(bytes@, start as int, width as nat),
{
    if start > bytes.len() || width > bytes.len() - start { return false; }
    let end = start + width;
    (start == 0 || bytes[start - 1] == 10u8)
        && (end == bytes.len() || bytes[end] == 10u8 ||
            (bytes[end] == 13u8 && end + 1 < bytes.len() && bytes[end + 1] == 10u8))
}

/// The byte search supplies literal match offsets. This decision owns their cardinality, order,
/// line boundaries, and the exact half-open interval used by the writer.
pub fn guide_edit(bytes: &[u8], reserved_count: usize, starts: &[usize], ends: &[usize],
    begin_width: usize, end_width: usize) -> (edit: GuideEdit)
    ensures
        match edit {
            GuideEdit::Append(separator) => reserved_count == 0 &&
                separator == (bytes.len() > 0 && bytes@[bytes.len()-1] != 10),
            GuideEdit::Malformed => reserved_count != 0 &&
                (reserved_count != 2 || starts.len() != 1 || ends.len() != 1),
            GuideEdit::InvalidLine => reserved_count == 2 && starts.len() == 1 && ends.len() == 1 &&
                !(starts@[0] < ends@[0] && standalone(bytes@, starts@[0] as int, begin_width as nat)
                    && standalone(bytes@, ends@[0] as int, end_width as nat)),
            GuideEdit::Replace(a, b) => reserved_count == 2 && starts.len() == 1 && ends.len() == 1 &&
                starts@[0] < ends@[0] && standalone(bytes@, starts@[0] as int, begin_width as nat)
                    && standalone(bytes@, ends@[0] as int, end_width as nat) &&
                a == starts@[0] && b == ends@[0] + end_width && a <= b <= bytes.len(),
        },
{
    if reserved_count == 0 {
        return GuideEdit::Append(bytes.len() != 0 && bytes[bytes.len()-1] != 10u8);
    }
    if reserved_count != 2 || starts.len() != 1 || ends.len() != 1 {
        return GuideEdit::Malformed;
    }
    let start = starts[0];
    let end = ends[0];
    if start >= end || !marker_line(bytes, start, begin_width) || !marker_line(bytes, end, end_width) {
        return GuideEdit::InvalidLine;
    }
    assert(end as int + end_width as int <= bytes.len());
    GuideEdit::Replace(start, end + end_width)
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
