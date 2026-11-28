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
use vstd::utf8::encode_utf8;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

#[path = "../examples/release_observation.rs"]
mod release_observation;

verus! {

#[cfg(verus_keep_ghost)]
fn catalog_admission_requires_the_declared_schema_and_skill_limits(
    producer: &[u8], renderer: &[u8], other: &[u8], name: &[u8],
)
    requires name@.len() > 63,
{
    use policy::catalog::{catalog_schema, manifest_schema, skill_description, skill_name, skill_position};
    let valid = catalog_schema(1);
    let unsupported = catalog_schema(2);
    let wrong_schema = manifest_schema(1, producer, renderer, renderer);
    let checked_renderer = manifest_schema(2, "chrono-instructions".as_bytes(), renderer, other);
    let too_long = skill_name(name);
    let short = [97u8];
    let adjacent = [97u8, 45u8, 45u8, 98u8];
    let valid_name = skill_name(&short);
    let doubled = skill_name(&adjacent);
    let valid_description = skill_description(true, 1024, false);
    let long_description = skill_description(true, 1025, false);
    let tagged_description = skill_description(true, 1, true);
    assert(valid && !unsupported && !wrong_schema);
    assert(checked_renderer == (renderer@ == other@));
    assert(skill_position(short@, 0));
    assert(!skill_position(adjacent@, 2));
    assert(!too_long && valid_name && !doubled);
    assert(valid_description && !long_description && !tagged_description);
}

#[cfg(verus_keep_ghost)]
fn dependency_cycle_wins_before_seen_and_missing() {
    use policy::catalog::{Dependency, dependency};
    let cycle = dependency::<u8>(true, true, None);
    let seen = dependency::<u8>(false, true, None);
    let missing = dependency::<u8>(false, false, None);
    let open = dependency(false, false, Some(3u8));
    assert(cycle is Cycle && seen is Visited && missing is Missing);
    assert(open == Dependency::Open(3u8));
}

#[cfg(verus_keep_ghost)]
fn successful_walk_has_each_root_and_only_its_prerequisites<'a>(
    input: &std::collections::BTreeMap<&'a String, &'a policy::catalog::Atom>,
    roots: &[&'a String],
    allowed: Set<&'a String>,
)
    requires vstd::laws_cmp::obeys_cmp::<&String>(),
        forall|key: &String| input@.contains_key(key) ==> &(#[trigger] input@[key]).id == key,
        policy::catalog::closed(input@, allowed), roots@.to_set().subset_of(allowed),
{
    if let Ok(order) = policy::catalog::walk(input, roots) {
        proof {
            let ids = order@.map(|_i, atom: &policy::catalog::Atom| &atom.id);
            assert(ids.no_duplicates());
            assert(roots@.to_set().subset_of(ids.to_set()));
            assert(ids.to_set().subset_of(allowed));
            assert forall|i: int, j: int| 0 <= i < order@.len()
                && 0 <= j < order@[i].requires@.len() implies
                ids.take(i).to_set().contains(#[trigger] &order@[i].requires@[j]) by {
                assert(policy::catalog::has_dependencies(input@, ids.take(i).to_set(), ids[i]));
                // The graph is indexed by the ID of its actual atom.
                assert(input@.contains_key(ids[i]));
            }
        }
    }
}

#[cfg(verus_keep_ghost)]
fn heading_depth_reserves_one_level_for_an_output_title() {
    use policy::catalog::{rendered_depth, section_depth};
    let first = section_depth(0, 1);
    let skipped = section_depth(0, 2);
    let sixth = section_depth(5, 6);
    let seventh = section_depth(6, 7);
    let untitled = rendered_depth(6, false);
    let titled = rendered_depth(6, true);
    assert(first && !skipped && sixth && !seventh);
    assert(untitled == Some(6u8) && titled.is_none());
}

#[cfg(verus_keep_ghost)]
fn registered_path_errors_prioritize_spelling_then_reserved_then_source(path: &[u8], source: bool) {
    use policy::host_paths::{registered_path,PathUse};
    let invalid = registered_path(path, false, source);
    assert(invalid is NonCanonical);
    let output = registered_path(path, true, false);
    let input = registered_path(path, true, true);
    assert((output is Reserved) ==> (input is Reserved));
    assert((input is Allowed) ==> (output is Allowed));
    assert((input is ExternalSource) ==> (output is Allowed));
}

#[cfg(verus_keep_ghost)]
fn component_boundaries_and_source_roots(prefix: &[u8], path: &[u8], boundary: usize)
    requires
        boundary == prefix.len(), boundary < path.len(),
        path@.subrange(0, boundary as int) == prefix@,
{
    use policy::host_paths::{directory_relation, canonical_component};
    let empty: [u8; 0] = [];
    let dot: [u8; 1] = [46];
    let double: [u8; 2] = [46,46];
    let triple: [u8; 3] = [46,46,46];
    let unnamed = canonical_component(&empty);
    let current = canonical_component(&dot);
    let parent = canonical_component(&double);
    let named = canonical_component(&triple);
    assert(!unnamed && !current && !parent && named);
    let (exact, related) = directory_relation(path, prefix);
    assert(!exact);
    assert(related == (path@[boundary as int] == 47u8));
    let (same, child) = directory_relation(prefix, prefix);
    assert(same && !child);
}

#[cfg(verus_keep_ghost)]
fn emitted_text_preserves_first_notice_and_both_bodies(
    first_id: &[u8], first: &[u8], second_id: &[u8], second: &[u8], notice: &[u8],
) {
    let mut out = Vec::new();
    policy::inherit::append_rule(&mut out, notice, first_id, first);
    assert(out@ =~= notice@ + seq![10u8, 10u8, 35u8, 35u8, 32u8] + first_id@ + seq![10u8, 10u8] + first@ + seq![10u8]);
    policy::inherit::append_rule(&mut out, notice, second_id, second);
    assert(out@ =~= notice@ + seq![10u8, 10u8, 35u8, 35u8, 32u8] + first_id@ + seq![10u8, 10u8] + first@
        + seq![10u8, 10u8, 35u8, 35u8, 32u8] + second_id@ + seq![10u8, 10u8] + second@ + seq![10u8]);
}

#[cfg(verus_keep_ghost)]
fn layout_emits_exact_bodies_in_order<'a>(
    first: &'a String, second: &'a String, first_text: &'a str, second_text: &'a str,
    ids: &'a [String],
)
    requires first != second, ids@.len() == 2, &ids@[0] == second, &ids@[1] == first,
        vstd::laws_cmp::obeys_cmp::<&String>(),
{
    broadcast use vstd::std_specs::btree::group_btree_axioms;
    broadcast use vstd::set::group_set_axioms;
    broadcast use vstd::seq::group_seq_axioms;
    broadcast use vstd::seq_lib::group_seq_properties;
    use policy::catalog::{Body, place_section};
    let mut available = std::collections::BTreeMap::new();
    let first_body = Body { id: first, text: first_text };
    let second_body = Body { id: second, text: second_text };
    available.insert(first, first_body);
    available.insert(second, second_body);
    let ghost full = available@;
    let mut ordered = Vec::new();
    let result = place_section(&mut available, &mut ordered, ids);
    assert(result is Ok);
    proof {
        assert(ids@.map(|_i, id: String| &id) =~= seq![second, first]);
        assert(full.dom() =~= set![first, second]);
        assert(available@.dom() =~= Set::<&String>::empty());
    }
    assert(ordered@ == seq![second_body, first_body]);
}

#[cfg(verus_keep_ghost)]
fn layout_stops_at_the_first_missing_placement<'a>(ids: &'a [String], body: policy::catalog::Body<'a>)
    requires ids@.len() >= 2, &ids@[0] == body.id, &ids@[1] != body.id,
        vstd::laws_cmp::obeys_cmp::<&String>(),
{
    use policy::catalog::place_section;
    let mut available = std::collections::BTreeMap::new();
    available.insert(body.id, body);
    let mut ordered = Vec::new();
    let result = place_section(&mut available, &mut ordered, ids);
    assert(result == Err::<(), &String>(&ids@[1]));
    assert(ordered@ == seq![body]);
}

#[cfg(verus_keep_ghost)]
fn empty_and_nonempty_bodies_keep_their_original_bytes<'a>(
    id: &'a String, empty: &'a str, text: &'a str,
)
    requires empty@.len() == 0, text@.len() > 0,
{
    use policy::catalog::{Body, record_body};
    let mut order = Vec::new();
    let original = Body { id, text };
    record_body(&mut order, Body { id, text: empty });
    record_body(&mut order, original);
    record_body(&mut order, Body { id, text: empty });
    assert(order@ == seq![original]);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn release_accepts_the_expected_observations_only() {
    let accepted = release_observation::accepts(true, true);
    assert(accepted);
    let refused_1 = release_observation::accepts(false, true);
    assert(!refused_1);
    let refused_2 = release_observation::accepts(true, false);
    assert(!refused_2);
}


/// The role that first claimed a name can never be displaced by the next role.
fn registered_role_collision() {
    use policy::catalog_admission::{claim_path, PathRole};
    let mut names = std::collections::BTreeSet::<u8>::new();
    let first = claim_path(&mut names, 1, PathRole::Control);
    assert(first is Ok);
    let control = claim_path(&mut names, 1, PathRole::Control);
    assert(control == Err::<(),&str>("source/control/alias overlap"));
    let input = claim_path(&mut names, 1, PathRole::Source);
    assert(input == Err::<(),&str>("source/control overlap"));
    let output = claim_path(&mut names, 1, PathRole::Output);
    assert(output == Err::<(),&str>("source/output/alias overlap"));
}

fn folded_name_or_file_identity(current: &String, old: &String, allow_same: bool) {
    use policy::catalog_admission::alias_conflict;
    let fresh = alias_conflict(None, current, allow_same);
    assert(fresh is None);
    let existing = alias_conflict(Some(old), current, allow_same);
    assert(existing == (if allow_same && current@ == old@ { None } else { Some(old) }));
}

fn first_sorted_candidate_can_only_be_a_descendant_at_a_separator(path: &[u8], other: &[u8]) {
    let yes = policy::catalog_admission::descendant_path(path,other);
    proof {
        if other.len() > path.len() {
            let prefix = other@.subrange(0, path.len() as int);
            assert((prefix =~= path@) == (forall|i:int| 0 <= i < path.len() ==> #[trigger] path@[i] == other@[i]));
        }
    }
    assert(yes == (other.len() > path.len() && other@[path.len() as int] == 47u8 &&
        forall|i:int| 0 <= i < path.len() ==> #[trigger] path@[i] == other@[i]));
}

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

// The native root checks this version string against the actual --version process output.
#[cfg(verus_keep_ghost)]
#[path = "generation.rs"]
mod generation;
