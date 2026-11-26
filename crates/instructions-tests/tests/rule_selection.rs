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

fn existing_registration_always_supplies_its_original_value(manifest: u64, input: Option<u64>) {
    use policy::catalog_admission::{registration, Registration};
    let choice = registration(Some(manifest), input);
    assert(choice == Registration::<u64, u64>::Retained(manifest));
}

fn missing_registration_requires_and_preserves_the_explicit_input(input: u64) {
    use policy::catalog_admission::{registration, Registration};
    let missing = registration::<u64, u64>(None, None);
    assert(missing is Missing);
    let choice = registration::<u64, u64>(None, Some(input));
    assert(choice == Registration::<u64, u64>::Initialize(input));
}

fn explicit_overrides_require_an_actual_registered_byte_match(selected: &[u8], saved: &[u8]) {
    use policy::catalog_admission::accepts_override;
    let inherited = accepts_override(None, None);
    let missing = accepts_override(Some(selected), None);
    let observed = accepts_override(Some(selected), Some(saved));
    assert(inherited && !missing);
    assert(observed == (selected@ == saved@));
}

fn missing_second_root_preserves_every_primary_byte(primary: Vec<u8>) {
    let ghost expected = primary@;
    let published = policy::consolidate_guides(primary, None);
    assert(published.is_ok() && published.unwrap()@ == expected);
}

fn both_root_guides_must_agree_on_the_actual_bytes(primary: Vec<u8>, other: &[u8]) {
    let ghost expected = primary@;
    let published = policy::consolidate_guides(primary, Some(other));
    assert(expected == other@ ==> published.is_ok() && published.unwrap()@ == expected);
    assert(expected != other@ ==> published.is_err());
}

fn expect(value: bool)
    requires value,
{ if !value { panic!(); } }

#[cfg_attr(not(verus_keep_ghost), test)]
fn host_guide_edits_only_the_declared_standalone_lines() {
    use policy::{GuideEdit, guide_edit};
    // Two-character delimiters represent the positions found in the host's bytes.
    let bytes = [120u8,10u8,60u8,98u8,13u8,10u8,113u8,10u8,60u8,101u8,13u8,10u8,121u8];
    let edit = guide_edit(&bytes, 2, &[2], &[8], 2, 2);
    match edit {
        GuideEdit::Replace(start, end) => expect(start == 2 && end == 10),
        _ => expect(false),
    }
    let empty = guide_edit(&[], 0, &[], &[], 2, 2);
    expect(matches!(empty, GuideEdit::Append(false)));
    let append = guide_edit(&[120u8], 0, &[], &[], 2, 2);
    expect(matches!(append, GuideEdit::Append(true)));
    let duplicate = guide_edit(&bytes, 3, &[2, 8], &[8], 2, 2);
    expect(matches!(duplicate, GuideEdit::Malformed));
    let reversed = guide_edit(&bytes, 2, &[8], &[2], 2, 2);
    expect(matches!(reversed, GuideEdit::InvalidLine));
    let inline = guide_edit(&bytes, 2, &[3], &[8], 1, 2);
    expect(matches!(inline, GuideEdit::InvalidLine));
    let eof = guide_edit(&[60u8,10u8,62u8], 2, &[0], &[2], 1, 1);
    expect(matches!(eof, GuideEdit::Replace(0, 3)));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn catalog_admission_preserves_unique_names_and_first_errors() {
    use policy::catalog_admission::*;
    use std::collections::{BTreeMap, BTreeSet};
    let mut ids = BTreeSet::new();
    let mut paths = BTreeSet::new();
    let first = output(&mut ids,&mut paths,1u8,2u8,1); expect(first.is_ok());
    let duplicate = output(&mut ids,&mut paths,1u8,2u8,0);
    expect(matches!(duplicate,Err(OutputError::DuplicateId)));
    let path = output(&mut ids,&mut paths,3u8,2u8,0);
    expect(matches!(path,Err(OutputError::DuplicatePath)));
    let empty = output(&mut ids,&mut paths,4u8,9u8,0);
    expect(matches!(empty,Err(OutputError::EmptyRoots)));
    let root = root_output(true,"CLAUDE.md".as_bytes()); expect(matches!(root,Ok(true)));
    let reserved = root_output(false,"CLAUDE.md".as_bytes()); expect(reserved.is_err());
    let missing = finish_roots(0); expect(missing.is_err());
    let one = finish_roots(1); expect(one.is_ok());
    let two = finish_roots(2); expect(two.is_err());
    let mut locales = BTreeSet::new();
    let good = locale(&mut locales,1u8,false,false); expect(good.is_ok());
    let repeated = locale(&mut locales,1u8,true,true); expect(matches!(repeated,Err(LocaleError::Duplicate)));
    let bad_frame = locale(&mut locales,2u8,true,false); expect(matches!(bad_frame,Err(LocaleError::EmptyFrame)));
    let mut atoms: BTreeMap<u8,u8> = BTreeMap::new();
    let new = register_atom(&mut atoms,1,20); expect(new);
    let known = register_atom(&mut atoms,1,30); expect(!known);
    let mut variants = BTreeSet::new();
    let unpublished = variant(&locales,&mut variants,3); expect(matches!(unpublished,Err(VariantError::Unregistered)));
    let declared = variant(&locales,&mut variants,1); expect(declared.is_ok());
    let duplicate_variant = variant(&locales,&mut variants,1); expect(matches!(duplicate_variant,Err(VariantError::Duplicate)));
}

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
