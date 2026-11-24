//! Independent expectations for explicit policy, source validity and projection safety.
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../instructions/src/rule_selection.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_instructions::rule_selection as policy;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {
fn expect(value: bool)
    requires value,
{ if !value { panic!(); } }

#[cfg_attr(not(verus_keep_ghost), test)]
fn upstream_rows_preserve_mode_and_identity() {
    use policy::{Mode,Row};
    expect(policy::needs_feed(&Mode::All,0));
    expect(policy::needs_feed(&Mode::Only,1));
    expect(!policy::needs_feed(&Mode::Only,0));
    expect(!policy::needs_feed(&Mode::Off,1));
    let id: &[u8] = &[1,2]; let other: &[u8] = &[1,3]; let body: &[u8] = &[4];
    expect(policy::schema(id,id)); expect(!policy::schema(id,other));
    expect(matches!(policy::row(&Mode::All,id,body,None,None),Row::Publish));
    expect(matches!(policy::row(&Mode::All,id,body,Some(id),None),Row::Omit));
    expect(matches!(policy::row(&Mode::All,id,body,Some(other),Some(other)),Row::Publish));
    expect(matches!(policy::row(&Mode::Only,id,body,None,None),Row::Omit));
    expect(matches!(policy::row(&Mode::Only,id,body,Some(id),None),Row::Publish));
    expect(matches!(policy::row(&Mode::Off,id,body,Some(id),None),Row::Omit));
    expect(matches!(policy::row(&Mode::Only,id,body,None,Some(id)),Row::Invalid));
    expect(matches!(policy::row(&Mode::All,&[],body,None,None),Row::Invalid));
    expect(matches!(policy::row(&Mode::All,id,&[],None,None),Row::Invalid));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn delimiter_at_any_offset_prevents_publication() {
    let marker: &[u8] = &[1,2];
    let leading: &[u8] = &[1,2,9];
    let middle: &[u8] = &[9,1,2,9];
    let trailing: &[u8] = &[9,1,2];
    let safe: &[u8] = &[9,1,3,9];
    proof {
        assert(policy::prefix_at(leading@,marker@,0));
        assert(policy::prefix_at(middle@,marker@,1));
        assert(policy::prefix_at(trailing@,marker@,1));
        assert forall|i:int| !#[trigger] policy::prefix_at(safe@,marker@,i) by {
            if policy::prefix_at(safe@,marker@,i) {
                assert(0 <= i <= 2);
                assert(safe@[i + 0] == marker@[0]);
                assert(safe@[i + 1] == marker@[1]);
                if i == 0 {} else if i == 1 {} else { assert(i == 2); }
            }
        }
    }
    expect(!policy::safe_projection(leading,marker));
    expect(!policy::safe_projection(middle,marker));
    expect(!policy::safe_projection(trailing,marker));
    expect(policy::safe_projection(safe,marker));
    expect(policy::safe_projection(&[],marker));
}
}
