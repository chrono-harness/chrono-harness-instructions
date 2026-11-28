//! Boundary expectations for the identity decision used by the event reader and formatter.
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../instructions/src/event_identity.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_instructions::event_identity as policy;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

#[cfg(not(verus_keep_ghost))]
use chrono_instructions::github_event as layout;
#[cfg(verus_keep_ghost)]
use policy::layout;

verus! {
fn expect(valid: bool)
    requires valid,
{
    if !valid { panic!(); }
}

// The last block's parser offset and content must describe the same bytes as the frame.
#[cfg(verus_keep_ghost)]
fn accepts_contiguous_top_level_summary(opening: usize, offset: usize, opening_len: usize,
    summary: &[u8], closing: usize)
    requires offset as int == opening as int + opening_len as int + 1,
        closing as int == offset as int + summary.len() as int + 1,
{
    expect(layout::visible_summary(Some((offset, summary)), opening, opening_len, summary, closing));
    expect(layout::paragraph_matches(Some((offset, summary)), offset, summary));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn event_layout_requires_visible_bytes_exact_offsets_and_a_blank_line() {
    let text: &[u8] = &[65u8, 0, 128];
    let wrong: &[u8] = &[65u8, 1, 128];
    proof { assert(text@[1] != wrong@[1]); }
    expect(layout::separated(&[]));
    expect(layout::separated(&[65u8, 10, 10]));
    expect(!layout::separated(&[10u8]));
    expect(!layout::separated(&[10u8, 65]));
    expect(!layout::separated(&[65u8, 10]));
    expect(!layout::paragraph_matches(None, 2, text));
    expect(!layout::paragraph_matches(Some((1usize, text)), 2, text));
    expect(!layout::paragraph_matches(Some((2usize, wrong)), 2, text));
    expect(layout::visible_summary(Some((2usize, text)), 0, 1, text, 6));
    expect(!layout::visible_summary(None, 0, 1, text, 6));
    expect(!layout::visible_summary(Some((2usize, wrong)), 0, 1, text, 6));
    expect(!layout::visible_summary(Some((2usize, text)), 1, 1, text, 6));
    expect(!layout::visible_summary(Some((2usize, text)), 0, 1, text, 5));
    expect(!layout::visible_summary(Some((2usize, text)), 0, 1, text, 7));
    expect(!layout::visible_summary(Some((0usize, text)), usize::MAX, 0, text, 4));
}

// State the independent cases using concatenation, arbitrary truncation and a
// differing byte. This covers every suffix, including an empty payload.
#[cfg(verus_keep_ghost)]
fn legacy_marker_followed_by_any_payload(frame: &[u8], tail: &[u8])
    requires frame@ == "<!-- chrono-agent:v3 ".spec_bytes() + tail@,
{
    let ghost marker = "<!-- chrono-agent:v3 ".spec_bytes();
    assert(frame@.subrange(0, marker.len() as int) =~= marker);
    expect(policy::uses_v3_frame(frame));
}

#[cfg(verus_keep_ghost)]
fn truncated_marker_or_different_byte(frame: &[u8], position: usize)
    requires ({
        let marker = "<!-- chrono-agent:v3 ".spec_bytes();
        frame.len() < marker.len() || (position < frame.len() && position < marker.len()
            && frame@[position as int] != marker[position as int])
    }),
{
    let ghost marker = "<!-- chrono-agent:v3 ".spec_bytes();
    proof {
        if frame.len() >= marker.len() {
            assert(frame@.subrange(0,marker.len() as int)[position as int] != marker[position as int]);
        }
    }
    expect(!policy::uses_v3_frame(frame));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn distinguishes_every_identity_role_and_both_delimiters() {
    let s = &[0u8,1]; let p = &[0u8,2]; let t = &[0u8,3]; let e = &[0u8,4];
    expect(matches!(policy::identity(s,p,t,e,Some(e)), policy::Identity::Valid));
    expect(matches!(policy::identity(s,&[45u8],t,e,None), policy::Identity::Valid));
    expect(matches!(policy::identity(s,&[45u8],t,e,Some(e)), policy::Identity::Valid));
    expect(matches!(policy::identity(s,p,t,e,Some(t)), policy::Identity::ClosingMismatch));
    let longer: &[u8] = &[0u8,4,0];
    let event: &[u8] = e;
    proof { assert(longer@.len() != event@.len()); }
    expect(matches!(policy::identity(s,p,t,event,Some(longer)), policy::Identity::ClosingMismatch));
    expect(matches!(policy::identity(s,s,t,e,Some(t)), policy::Identity::ClosingMismatch));
    expect(matches!(policy::identity(s,s,t,e,Some(e)), policy::Identity::ReusedId));
    expect(matches!(policy::identity(s,p,s,e,Some(e)), policy::Identity::ReusedId));
    expect(matches!(policy::identity(s,p,t,s,Some(s)), policy::Identity::ReusedId));
    expect(matches!(policy::identity(s,p,p,e,Some(e)), policy::Identity::ReusedId));
    expect(matches!(policy::identity(s,p,t,p,Some(p)), policy::Identity::ReusedId));
    expect(matches!(policy::identity(s,p,t,t,Some(t)), policy::Identity::ReusedId));
}
#[cfg_attr(not(verus_keep_ghost), test)]
fn marker_inventory_admits_only_one_ordered_candidate() {
    use layout::{Frame, Marker};
    expect(matches!(layout::select_frame(&[]), Frame::MissingOrAmbiguous));
    expect(matches!(layout::select_frame(&[Marker::Legacy(8)]), Frame::Legacy(8)));
    expect(matches!(layout::select_frame(&[Marker::Opening(8)]), Frame::MissingOrAmbiguous));
    expect(matches!(layout::select_frame(&[Marker::Opening(8), Marker::Closing(34)]),
        Frame::Framed(8, 34)));
    expect(matches!(layout::select_frame(&[Marker::Closing(8), Marker::Opening(34)]),
        Frame::MissingOrAmbiguous));
    expect(matches!(layout::select_frame(&[Marker::Legacy(8), Marker::Legacy(34)]),
        Frame::MissingOrAmbiguous));
    expect(matches!(layout::select_frame(&[Marker::Legacy(2), Marker::Opening(8), Marker::Closing(34)]),
        Frame::MissingOrAmbiguous));
    expect(matches!(layout::select_frame(&[Marker::Opening(8), Marker::Closing(34), Marker::Opening(40)]),
        Frame::MissingOrAmbiguous));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn event_admission_keeps_terminator_and_layout_errors_in_order() {
    use layout::FrameLayout;
    let summary: &[u8] = &[67u8];
    let paragraph = Some((2usize, summary));
    expect(layout::body_end(&[10u8, 10]) == Some(1));
    expect(layout::body_end(&[13u8, 10]).is_none());
    expect(layout::body_end(&[]) == Some(0));
    expect(layout::is_top_level(0));
    expect(!layout::is_top_level(1));
    expect(matches!(layout::framed_layout(paragraph, 0, 4, None), FrameLayout::MissingSummary));
    expect(matches!(layout::framed_layout(None, 0, 4, Some((1, None))), FrameLayout::MissingClosing));
    expect(matches!(layout::framed_layout(None, 0, 4, Some((1, Some(summary)))),
        FrameLayout::HiddenOrMisplaced));
    expect(matches!(layout::framed_layout(paragraph, 0, 4, Some((1, Some(summary)))),
        FrameLayout::Visible));
}

#[cfg(verus_keep_ghost)]
fn opening_marker_keeps_the_observed_start(start: usize, raw: &[u8], rest: &[u8])
    requires raw@ == "<!-- chrono-agent:".spec_bytes() + rest@,
{
    let ghost prefix = "<!-- chrono-agent:".spec_bytes();
    assert(raw@.subrange(0, prefix.len() as int) =~= prefix);
    expect(matches!(layout::html_marker(start, raw), Some(layout::Marker::Opening(at)) if at == start));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn roundtrip_compares_every_field() {
    let value: &[u8] = &[1u8, 2];
    let changed: &[u8] = &[1u8, 3];
    let original = [value, value, value, value, value, value, value, value, value];
    proof { assert forall|j: int| 0 <= j < 9 implies (#[trigger] original@[j])@ == original@[j]@ by {} }
    expect(layout::same_fields(&original, &original));
    let first = [changed, value, value, value, value, value, value, value, value];
    let last = [value, value, value, value, value, value, value, value, changed];
    proof {
        assert(first@[0]@ != original@[0]@);
        assert(last@[8]@ != original@[8]@);
    }
    expect(!layout::same_fields(&original, &first));
    expect(!layout::same_fields(&original, &last));
}

}
