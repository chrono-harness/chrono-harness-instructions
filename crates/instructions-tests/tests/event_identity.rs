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
use vstd::vpanic as panic;

verus! {
fn expect(valid: bool)
    requires valid,
{
    if !valid { panic!(); }
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
}
