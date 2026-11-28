#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::utf8::encode_utf8;

verus! {
pub(crate) fn catalog_schema(version: u32) -> (accepted: bool)
    ensures accepted == (version == 1),
{
    version == 1
}

pub(crate) fn manifest_schema(
    version: u32, producer: &[u8], renderer: &[u8], expected_renderer: &[u8],
) -> (accepted: bool)
    ensures accepted == (version == 2
        && producer@ == encode_utf8("chrono-instructions"@)
        && renderer@ == expected_renderer@),
{
    version == 2 && super::verified_bytes::equal(producer, "chrono-instructions".as_bytes())
        && super::verified_bytes::equal(renderer, expected_renderer)
}

pub(crate) open spec fn skill_character(byte: u8) -> bool {
    97 <= byte <= 122 || 48 <= byte <= 57 || byte == 45
}

pub(crate) open spec fn skill_position(name: Seq<u8>, i: int) -> bool {
    skill_character(name[i]) && (i == 0 || name[i - 1] != 45 || name[i] != 45)
}

pub(crate) fn skill_name(name: &[u8]) -> (accepted: bool)
    ensures accepted == (0 < name@.len() <= 63
        && name@[0] != 45 && name@[name@.len() - 1] != 45
        && forall|i: int| 0 <= i < name@.len() ==> #[trigger] skill_position(name@, i)),
{
    // ASCII ranges for a-z, 0-9, and hyphen.
    if name.is_empty() || name.len() > 63 || name[0] == 45 || name[name.len() - 1] == 45 {
        return false;
    }
    let mut i = 0;
    while i < name.len()
        invariant i <= name@.len(), 0 < name@.len() <= 63,
            name@[0] != 45, name@[name@.len() - 1] != 45,
            forall|j: int| 0 <= j < i ==> #[trigger] skill_position(name@, j),
        decreases name.len() - i,
    {
        let byte = name[i];
        if !(97 <= byte && byte <= 122 || 48 <= byte && byte <= 57 || byte == 45)
            || (i > 0 && name[i - 1] == 45 && byte == 45)
        {
            assert(!skill_position(name@, i as int));
            return false;
        }
        assert(skill_position(name@, i as int));
        i += 1;
    }
    true
}

pub(crate) fn skill_description(single_line: bool, length: usize, has_angle: bool) -> (accepted: bool)
    ensures accepted == (single_line && length <= 1024 && !has_angle),
{
    single_line && length <= 1024 && !has_angle
}

pub(crate) fn section_depth(previous: u8, depth: u8) -> (accepted: bool)
    ensures accepted == (1 <= depth <= 6 && depth as int <= previous as int + 1),
{
    1 <= depth && depth <= 6 && (depth as u16) <= previous as u16 + 1
}

pub(crate) fn rendered_depth(depth: u8, has_title: bool) -> (effective: Option<u8>)
    ensures match effective {
        Some(heading) => heading as int == depth as int + (if has_title { 1int } else { 0int })
            && heading <= 6,
        None => depth as int + (if has_title { 1int } else { 0int }) > 6,
    },
{
    let effective = depth as u16 + if has_title { 1u16 } else { 0u16 };
    if effective <= 6 { Some(effective as u8) } else { None }
}

pub(crate) enum Dependency<T> {
    Cycle,
    Visited,
    Missing,
    Open(T),
}

pub(crate) fn dependency<T>(active: bool, visited: bool, entry: Option<T>)
    -> (step: Dependency<T>)
    ensures step == if active { Dependency::Cycle }
        else if visited { Dependency::Visited }
        else { match entry { Some(atom) => Dependency::Open(atom), None => Dependency::Missing } },
{
    if active {
        return Dependency::Cycle;
    }
    if visited {
        return Dependency::Visited;
    }
    match entry {
        Some(atom) => Dependency::Open(atom),
        None => Dependency::Missing,
    }
}

pub(crate) enum WalkError<'a> {
    Cycle(&'a String, Vec<&'a String>),
    Missing(&'a String, Vec<&'a String>),
}

#[derive(Clone, Copy)]
struct Frame<'a> {
    id: &'a String,
    atom: &'a Atom,
    next: usize,
}

// Each active frame contributes its unread edges and one step to close it.
spec fn remaining(stack: Seq<Frame>) -> int
    decreases stack.len(),
{
    if stack.len() == 0 { 0 } else {
        remaining(stack.drop_last()) + stack.last().atom.requires@.len() - stack.last().next + 1
    }
}

proof fn nonnegative_remaining(stack: Seq<Frame>)
    requires forall|i: int| 0 <= i < stack.len() ==> #[trigger] stack[i].next <= stack[i].atom.requires@.len(),
    ensures 0 <= remaining(stack),
    decreases stack.len(),
{
    if stack.len() > 0 {
        assert forall|i: int| 0 <= i < stack.drop_last().len() implies
            #[trigger] stack.drop_last()[i].next <= stack.drop_last()[i].atom.requires@.len() by {
            assert(stack.drop_last()[i] == stack[i]);
        }
        nonnegative_remaining(stack.drop_last());
        assert(stack.last() == stack[stack.len() - 1]);
    }
}

pub(crate) open spec fn has_dependencies(atoms: Map<&String, &Atom>, included: Set<&String>, id: &String) -> bool {
    atoms.contains_key(id)
        && forall|position: int| 0 <= position < atoms[id].requires@.len() ==>
            included.contains(#[trigger] (&atoms[id].requires@[position]))
}

pub(crate) open spec fn closed(atoms: Map<&String, &Atom>, included: Set<&String>) -> bool {
    forall|id: &String| included.contains(id) ==> #[trigger] has_dependencies(atoms, included, id)
}

fn error_path<'a>(stack: &[Frame<'a>]) -> (path: Vec<&'a String>)
    ensures path@ == stack@.map(|_i, frame: Frame<'a>| frame.id),
{
    let mut path = Vec::new();
    let mut i = 0;
    while i < stack.len()
        invariant i <= stack@.len(), path@.len() == i,
            forall|j: int| 0 <= j < i ==> path@[j] == stack@[j].id,
        decreases stack.len() - i,
    {
        path.push(stack[i].id);
        i += 1;
    }
    path
}

/// Depth-first postorder in root and requirement order. The active path lives on the heap.
pub(crate) fn walk<'a>(atoms: &BTreeMap<&'a String, &'a Atom>, roots: &[&'a String])
    -> (result: Result<Vec<&'a Atom>, WalkError<'a>>)
    requires vstd::laws_cmp::obeys_cmp::<&String>(),
        forall|id: &String| atoms@.contains_key(id) ==> &(#[trigger] atoms@[id]).id == id,
    ensures match result {
        Ok(order) => {
            let ids = order@.map(|_i, atom: &Atom| &atom.id);
            &&& ids.no_duplicates()
            &&& roots@.to_set().subset_of(ids.to_set())
            &&& forall|i: int| 0 <= i < order@.len() ==> {
                &&& atoms@.contains_pair(ids[i], #[trigger] order@[i])
                &&& has_dependencies(atoms@, ids.take(i).to_set(), ids[i])
            }
            &&& forall|candidate: Set<&String>| roots@.to_set().subset_of(candidate)
                && #[trigger] closed(atoms@, candidate) ==> ids.to_set().subset_of(candidate)
        },
        Err(WalkError::Cycle(id, path)) => path@.contains(id),
        Err(WalkError::Missing(id, path)) => !atoms@.contains_key(id) && !path@.contains(id),
    },
{
    broadcast use vstd::std_specs::btree::group_btree_axioms;
    broadcast use vstd::set::group_set_axioms;
    broadcast use vstd::seq::group_seq_axioms;
    broadcast use vstd::seq_lib::group_seq_properties;
    let mut root = 0;
    let mut stack: Vec<Frame> = Vec::new();
    let mut active: BTreeSet<&String> = BTreeSet::new();
    let mut seen: BTreeSet<&String> = BTreeSet::new();
    let mut order: Vec<&Atom> = Vec::new();
    while root < roots.len() || !stack.is_empty()
        invariant
            vstd::laws_cmp::obeys_cmp::<&String>(),
            root <= roots@.len(),
            0 <= remaining(stack@),
            atoms@.dom().finite(),
            forall|id: &String| atoms@.contains_key(id) ==> &(#[trigger] atoms@[id]).id == id,
            seen@.subset_of(atoms@.dom()),
            active@.subset_of(seen@),
            active@ == stack@.map(|_i, frame: Frame| frame.id).to_set(),
            stack@.map(|_i, frame: Frame| frame.id).no_duplicates(),
            forall|i: int| 0 <= i < stack@.len() ==> {
                let frame = #[trigger] stack@[i];
                &&& atoms@.contains_pair(frame.id, frame.atom)
                &&& frame.next <= frame.atom.requires@.len()
                &&& forall|j: int| 0 <= j < frame.next ==> {
                    let required = #[trigger] &frame.atom.requires@[j];
                    if j + 1 == frame.next && i + 1 < stack@.len() {
                        required == stack@[i + 1].id
                    } else {
                        seen@.difference(active@).contains(required)
                    }
                }
                &&& (i + 1 < stack@.len() ==> frame.next > 0)
            },
            roots@.take(root as int).to_set().subset_of(seen@),
            ({
                let ids = order@.map(|_i, atom: &Atom| &atom.id);
                &&& ids.no_duplicates()
                &&& ids.to_set() == seen@.difference(active@)
                &&& forall|i: int| 0 <= i < order@.len() ==> {
                    &&& atoms@.contains_pair(ids[i], #[trigger] order@[i])
                    &&& has_dependencies(atoms@, ids.take(i).to_set(), ids[i])
                }
            }),
            forall|candidate: Set<&String>| roots@.to_set().subset_of(candidate)
                && #[trigger] closed(atoms@, candidate) ==> seen@.subset_of(candidate),
        decreases atoms@.dom().difference(seen@).len(),
            roots@.len() as int - root + remaining(stack@),
    {
        let ghost old_stack = stack@;
        let ghost old_seen = seen@;
        let ghost old_active = active@;
        let ghost old_order = order@;
        let ghost old_root = root;
        let ghost old_frames = old_stack.map(|_i, entry: Frame| entry.id);
        let ghost parent = if old_stack.len() == 0 { None } else { Some(old_stack.last()) };
        let id;
        if stack.is_empty() {
            id = roots[root];
            root += 1;
        } else {
            let top = stack.len() - 1;
            let frame = stack[top];
            if frame.next == frame.atom.requires.len() {
                stack.pop();
                active.remove(&frame.id);
                order.push(frame.atom);
                proof {
                    assert(stack@ =~= old_stack.drop_last());
                    let previous = old_order.map(|_i, atom: &Atom| &atom.id);
                    let ids = order@.map(|_i, atom: &Atom| &atom.id);
                    assert(ids =~= previous.push(frame.id));
                    let frames = stack@.map(|_i, entry: Frame| entry.id);
                    assert(old_frames =~= frames.push(frame.id));
                    assert(frames =~= old_frames.drop_last());
                    assert(frames.no_duplicates()) by {
                        assert forall|i: int, j: int| 0 <= i < j < frames.len()
                            implies frames[i] != frames[j] by {
                            assert(old_frames[i] == frames[i] && old_frames[j] == frames[j]);
                        }
                    }
                    assert(active@ == old_active.remove(frame.id));
                    assert forall|i: int| 0 <= i < frames.len() implies frames[i] != frame.id by {
                        assert(old_stack.map(|_i, entry: Frame| entry.id)[i] == frames[i]);
                        assert(old_stack.map(|_i, entry: Frame| entry.id).last() == frame.id);
                    }
                    assert(!frames.contains(frame.id));
                    frames.lemma_push_to_set_commute(frame.id);
                    previous.lemma_push_to_set_commute(frame.id);
                    assert(active@ =~= frames.to_set());
                    assert(old_active.contains(frame.id));
                    assert(seen@.contains(frame.id));
                    assert(previous.to_set() == old_seen.difference(old_active));
                    assert(!previous.to_set().contains(frame.id));
                    assert(ids.no_duplicates());
                    assert(ids.to_set() =~= seen@.difference(active@)) by {
                        assert forall|key: &String| ids.to_set().contains(key)
                            <==> seen@.difference(active@).contains(key) by {
                            assert(ids.to_set().contains(key) == (previous.to_set().contains(key) || key == frame.id));
                        }
                    }
                    assert(remaining(old_stack) == remaining(stack@) + 1);
                    assert forall|i: int| 0 <= i < stack@.len() implies
                        #[trigger] stack@[i].next <= stack@[i].atom.requires@.len() by {
                        assert(stack@[i] == old_stack[i]);
                    }
                    nonnegative_remaining(stack@);
                    assert forall|i: int| 0 <= i < order@.len() implies
                        atoms@.contains_pair(ids[i], #[trigger] order@[i])
                            && has_dependencies(atoms@, ids.take(i).to_set(), ids[i]) by {
                        if i < old_order.len() {
                            assert(ids.take(i) =~= previous.take(i));
                        } else {
                            assert(ids.take(i) =~= previous);
                            assert forall|j: int| 0 <= j < frame.atom.requires@.len() implies
                                previous.to_set().contains(#[trigger] (&frame.atom.requires@[j])) by {
                                // Completed children precede their caller.
                            }
                        }
                    }
                }
                continue;
            }
            id = &frame.atom.requires[frame.next];
            stack.pop();
            stack.push(Frame { next: frame.next + 1, ..frame });
            proof {
                assert(stack@.drop_last() =~= old_stack.drop_last());
                assert(remaining(stack@) == remaining(old_stack) - 1);
            }
        }
        let ghost advanced = stack@;
        proof {
            assert(advanced.map(|_i, frame: Frame| frame.id) =~= old_frames);
            assert forall|i: int| 0 <= i < advanced.len() implies
                #[trigger] advanced[i].next <= advanced[i].atom.requires@.len() by {
                if i + 1 < advanced.len() { assert(advanced[i] == old_stack[i]); }
            }
            nonnegative_remaining(advanced);
        }
        let in_path = active.contains(&id);
        let visited = seen.contains(&id);
        let entry = atoms.get(&id);
        assert(in_path == active@.contains(id));
        assert(visited == seen@.contains(id));
        assert(entry.is_some() == atoms@.contains_key(id));
        match dependency(in_path, visited, entry) {
            Dependency::Cycle => {
                let path = error_path(&stack);
                assert(path@ =~= old_frames);
                assert(path@.to_set().contains(id));
                return Err(WalkError::Cycle(id, path));
            },
            Dependency::Visited => {},
            Dependency::Missing => return Err(WalkError::Missing(id, error_path(&stack))),
            Dependency::Open(atom) => {
                proof {
                    assert(atoms@.contains_pair(id, *atom));
                    atoms@.dom().lemma_set_insert_diff_decreases(seen@, id);
                }
                active.insert(id);
                seen.insert(id);
                stack.push(Frame { id, atom: *atom, next: 0 });
                proof {
                    let frames = stack@.map(|_i, entry: Frame| entry.id);
                    assert(frames =~= old_frames.push(id));
                    old_frames.lemma_push_to_set_commute(id);
                    assert(!old_frames.contains(id));
                    assert(frames.no_duplicates());
                    assert(stack@.drop_last() =~= advanced);
                    assert(remaining(stack@) == remaining(advanced) + atom.requires@.len() + 1);
                    assert forall|candidate: Set<&String>| roots@.to_set().subset_of(candidate)
                        && #[trigger] closed(atoms@, candidate) implies seen@.subset_of(candidate) by {
                        assert(old_seen.subset_of(candidate));
                        if let Some(frame) = parent {
                            assert(old_frames.last() == frame.id);
                            assert(old_frames.contains(frame.id));
                            assert(old_active.contains(frame.id));
                            assert(old_seen.contains(frame.id));
                            assert(has_dependencies(atoms@, candidate, frame.id));
                            assert(candidate.contains(&frame.atom.requires@[frame.next as int]));
                        } else {
                            assert(roots@[old_root as int] == id);
                        }
                    }
                }
            }
        }
    }
    proof {
        assert(roots@.take(root as int) =~= roots@);
        assert(active@ =~= Set::<&String>::empty());
    }
    Ok(order)
}

/// Resolve a requested layout before considering dependency order.
pub(crate) fn layout_for<'a>(id: Option<&'a String>, layouts: &[Layout])
    -> (choice: Result<Option<usize>, &'a String>)
    ensures match choice {
        Ok(None) => id is None,
        Ok(Some(i)) => id is Some && i < layouts@.len()
            && encode_utf8(layouts@[i as int].id@) == encode_utf8(id.unwrap()@)
            && forall|j:int| 0 <= j < i ==> encode_utf8(layouts@[j].id@) != encode_utf8(id.unwrap()@),
        Err(missing) => id == Some(missing) && forall|j:int| 0 <= j < layouts@.len() ==>
            encode_utf8(layouts@[j].id@) != encode_utf8(missing@),
    },
{
    let Some(requested) = id else { return Ok(None); };
    let mut i = 0;
    while i < layouts.len()
        invariant id == Some(requested), i <= layouts@.len(),
            forall|j:int| 0 <= j < i ==> encode_utf8(layouts@[j].id@) != encode_utf8(requested@),
        decreases layouts.len() - i,
    {
        if super::verified_bytes::equal(layouts[i].id.as_str().as_bytes(), requested.as_str().as_bytes()) {
            return Ok(Some(i));
        }
        i += 1;
    }
    Err(requested)
}

#[cfg_attr(not(verus_keep_ghost), derive(Serialize))]
#[derive(Clone, Copy)]
pub(crate) struct Body<'a> {
    pub id: &'a String,
    pub text: &'a str,
}

/// Preserve each selected body exactly. A zero-length variant contributes no body.
pub(crate) fn record_body<'a>(bodies: &mut Vec<Body<'a>>, body: Body<'a>)
    ensures bodies@ == if body.text@.len() == 0 { old(bodies)@ } else { old(bodies)@.push(body) },
{
    if !body.text.is_empty() {
        bodies.push(body);
    }
}

/// A section appends the resolved body of each placement, in declaration order.
pub(crate) fn place_section<'a>(
    available: &mut BTreeMap<&'a String, Body<'a>>,
    ordered: &mut Vec<Body<'a>>,
    ids: &'a [String],
) -> (out: Result<(), &'a String>)
    requires vstd::laws_cmp::obeys_cmp::<&String>(),
    ensures
        ordered@.take(old(ordered)@.len() as int) == old(ordered)@,
        ({
            let count = ordered@.len() - old(ordered)@.len();
            let resolved = ids@.take(count).map(|_i, id: String| &id).to_set();
            &&& 0 <= count <= ids@.len()
            &&& forall|i: int| 0 <= i < count ==> {
                let key = #[trigger] &ids@[i];
                old(available)@.contains_key(key)
                    && ordered@[old(ordered)@.len() + i] == old(available)@[key]
            }
            &&& available@ == old(available)@.remove_keys(resolved)
            &&& match out {
                Ok(()) => count == ids@.len(),
                Err(id) => count < ids@.len() && id == &ids@[count]
                    && !available@.contains_key(id),
            }
        }),
{
    broadcast use vstd::std_specs::btree::group_btree_axioms;
    broadcast use vstd::seq::group_seq_axioms;
    broadcast use vstd::seq_lib::group_seq_properties;
    broadcast use vstd::set::group_set_axioms;
    let ghost initial = available@;
    let ghost previous = ordered@;
    let mut i = 0;
    while i < ids.len()
        invariant
            vstd::laws_cmp::obeys_cmp::<&String>(),
            initial == old(available)@, previous == old(ordered)@,
            i <= ids@.len(),
            ordered@.len() == previous.len() + i,
            ordered@.take(previous.len() as int) == previous,
            forall|j: int| 0 <= j < i ==> {
                let key = #[trigger] &ids@[j];
                initial.contains_key(key) && ordered@[previous.len() + j] == initial[key]
            },
            available@ == initial.remove_keys(ids@.take(i as int).map(|_i, id: String| &id).to_set()),
        decreases ids.len() - i,
    {
        let id = &ids[i];
        let ghost before = available@;
        match available.remove(&id) {
            Some(body) => {
                assert(before.contains_pair(id, body));
                ordered.push(body);
            }
            None => {
                assert(!before.contains_key(id));
                assert(available@ =~= before);
                return Err(id);
            }
        }
        proof {
            let prefix = ids@.take(i as int).map(|_i, id: String| &id);
            let next = ids@.take(i + 1).map(|_i, id: String| &id);
            assert(next =~= prefix.push(id));
            prefix.lemma_push_to_set_commute(id);
            assert(available@ =~= initial.remove_keys(next.to_set()));
        }
        i += 1;
    }
    Ok(())
}

}

#[cfg(not(verus_keep_ghost))]
use crate::rule_selection::catalog_admission::{self, LocaleError, OutputError, VariantError};
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[cfg(not(verus_keep_ghost))]
use std::path::Path;

#[cfg(not(verus_keep_ghost))]
pub(super) const DEFAULT_CATALOG: &str = include_str!("../../../assets/instructions/catalog.json");
#[cfg(not(verus_keep_ghost))]
pub(super) const DEFAULT_MANIFEST: &str =
    include_str!("../../../assets/instructions/default-manifest.json");
#[cfg(not(verus_keep_ghost))]
pub(super) const RENDER: &str = "atomic-rules/relative-alias/v3";

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Manifest {
    pub schema_version: u32,
    pub producer: String,
    pub render: String,
    pub catalog: String,
    pub host_context: String,
    pub outputs: Vec<Output>,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Output {
    pub id: String,
    pub path: String,
    pub format: Format,
    pub locale: String,
    pub roots: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill: Option<Skill>,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum Format {
    RootGuide,
    Markdown,
    Skill,
}
#[cfg(not(verus_keep_ghost))]
impl Format {
    fn label(self) -> &'static str {
        match self {
            Self::RootGuide => "root-guide",
            Self::Markdown => "markdown",
            Self::Skill => "skill",
        }
    }
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Skill {
    pub name: String,
    pub description: String,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Catalog {
    pub schema_version: u32,
    pub locales: Vec<Locale>,
    pub atoms: Vec<Atom>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layouts: Vec<Layout>,
}
#[cfg_attr(not(verus_keep_ghost), derive(Deserialize, Serialize))]
#[cfg_attr(not(verus_keep_ghost), serde(deny_unknown_fields))]
pub(crate) struct Layout {
    pub id: String,
    pub sections: Vec<Section>,
}
#[cfg_attr(not(verus_keep_ghost), derive(Deserialize, Serialize))]
#[cfg_attr(not(verus_keep_ghost), serde(deny_unknown_fields))]
pub(crate) struct Section {
    pub depth: u8,
    #[cfg(not(verus_keep_ghost))]
    pub titles: Vec<Title>,
    pub atoms: Vec<String>,
}
#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Title {
    pub locale: String,
    pub text: String,
}
#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Locale {
    pub id: String,
    pub root_frame: String,
    pub projection_notice: String,
}
#[cfg_attr(not(verus_keep_ghost), derive(Deserialize, Serialize))]
#[cfg_attr(not(verus_keep_ghost), serde(deny_unknown_fields))]
pub(crate) struct Atom {
    pub id: String,
    pub requires: Vec<String>,
    #[cfg(not(verus_keep_ghost))]
    pub variants: Vec<Variant>,
}
#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Variant {
    pub locale: String,
    pub source: Content,
}
#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub(super) enum Content {
    Inline { text: String },
    File { path: String },
}

#[cfg(not(verus_keep_ghost))]
pub(super) fn parse<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
    serde_json::from_slice(bytes).map_err(|e| e.to_string())
}
#[cfg(not(verus_keep_ghost))]
pub(super) fn bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    let mut result = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    result.push(b'\n');
    Ok(result)
}
#[cfg(not(verus_keep_ghost))]
fn identity(s: &str) -> Result<(), String> {
    if s.is_empty()
        || !s
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    {
        return Err(format!(
            "invalid identity {s:?}; use ASCII letters, digits, '.', '_' or '-'"
        ));
    }
    Ok(())
}
#[cfg(not(verus_keep_ghost))]
fn single_line(s: &str) -> bool {
    !s.trim().is_empty()
        && !s
            .chars()
            .any(|c| c.is_control() || matches!(c, '\u{2028}' | '\u{2029}'))
}
#[cfg(not(verus_keep_ghost))]
pub(super) fn content_valid(s: &str) -> Result<(), String> {
    if s.contains("<!-- chrono-instructions") {
        return Err("content contains reserved marker prefix <!-- chrono-instructions".into());
    }
    Ok(())
}
#[cfg(not(verus_keep_ghost))]
impl Skill {
    fn validate(&self) -> Result<(), String> {
        let name = &self.name;
        if !skill_name(name.as_bytes()) {
            return Err(format!("invalid skill name {name:?}"));
        }
        if !skill_description(
            single_line(&self.description),
            self.description.chars().count(),
            self.description.contains(['<', '>']),
        ) {
            return Err("skill description must be nonempty, single-line, <=1024 characters, without controls or angle brackets".into());
        }
        Ok(())
    }
    fn header(&self) -> String {
        // JSON quoted strings are YAML double-quoted scalars. Controls are rejected.
        format!(
            "---\nname: {}\ndescription: {}\n---\n",
            serde_json::to_string(&self.name).unwrap(),
            serde_json::to_string(&self.description).unwrap()
        )
    }
}
#[cfg(not(verus_keep_ghost))]
impl Manifest {
    pub fn validate(&self) -> Result<(), String> {
        if !manifest_schema(
            self.schema_version,
            self.producer.as_bytes(),
            self.render.as_bytes(),
            RENDER.as_bytes(),
        ) {
            return Err("unsupported registration: expected schema 2 / chrono-instructions / atomic-rules/relative-alias/v3".into());
        }
        let mut ids = BTreeSet::new();
        let mut paths = BTreeSet::new();
        let mut roots = 0;
        for output in &self.outputs {
            identity(&output.id)?;
            catalog_admission::output(
                &mut ids,
                &mut paths,
                &output.id,
                &output.path,
                output.roots.len(),
            )
            .map_err(|e| match e {
                OutputError::DuplicateId => format!("duplicate output ID {}", output.id),
                OutputError::DuplicatePath => format!("duplicate output path {}", output.path),
                OutputError::EmptyRoots => format!("output {}: roots must not be empty", output.id),
            })?;
            if let Some(title) = &output.title {
                if !single_line(title) {
                    return Err(format!(
                        "output {}: title must be nonempty single-line text",
                        output.id
                    ));
                }
                content_valid(title)?;
            }
            match (&output.format, &output.skill) {
                (Format::Skill, Some(skill)) => {
                    skill
                        .validate()
                        .map_err(|e| format!("output {}: {e}", output.id))?;
                    let path = Path::new(&output.path);
                    if path.file_name().and_then(|s| s.to_str()) != Some("SKILL.md")
                        || path
                            .parent()
                            .and_then(|p| p.file_name())
                            .and_then(|s| s.to_str())
                            != Some(&skill.name)
                    {
                        return Err(format!(
                            "output {}: skill path must end in {}/SKILL.md",
                            output.id, skill.name
                        ));
                    }
                }
                (Format::Skill, None) => {
                    return Err(format!("output {}: missing skill metadata", output.id));
                }
                (_, Some(_)) => {
                    return Err(format!(
                        "output {}: only skill format accepts skill metadata",
                        output.id
                    ));
                }
                _ => {}
            }
            roots += usize::from(catalog_admission::root_output(
                output.format == Format::RootGuide,
                output.path.as_bytes(),
            )?);
        }
        catalog_admission::finish_roots(roots).map_err(str::to_owned)
    }
    pub fn root(&self) -> &Output {
        self.outputs
            .iter()
            .find(|o| o.format == Format::RootGuide)
            .unwrap()
    }
}
#[cfg(not(verus_keep_ghost))]
impl Catalog {
    pub fn validate(&self) -> Result<(), String> {
        if !catalog_schema(self.schema_version) {
            return Err("unsupported catalog schema_version (expected 1)".into());
        }
        let mut locales = BTreeSet::new();
        for locale in &self.locales {
            identity(&locale.id)?;
            catalog_admission::locale(
                &mut locales,
                &locale.id,
                locale.root_frame.trim().is_empty(),
                locale.projection_notice.trim().is_empty(),
            )
            .map_err(|e| match e {
                LocaleError::Duplicate => format!("duplicate locale {}", locale.id),
                LocaleError::EmptyFrame => format!("locale {}: empty frame/notice", locale.id),
            })?;
            content_valid(&locale.root_frame)?;
            content_valid(&locale.projection_notice)?;
        }
        let mut atoms = BTreeMap::new();
        for atom in &self.atoms {
            identity(&atom.id)?;
            if !catalog_admission::register_atom(&mut atoms, &atom.id, atom) {
                return Err(format!("duplicate atom {}", atom.id));
            }
            let mut variants = BTreeSet::new();
            for variant in &atom.variants {
                catalog_admission::variant(&locales, &mut variants, &variant.locale).map_err(
                    |e| match e {
                        VariantError::Unregistered => {
                            format!("atom {}: unregistered locale {}", atom.id, variant.locale)
                        }
                        VariantError::Duplicate => {
                            format!("atom {}: duplicate variant {}", atom.id, variant.locale)
                        }
                    },
                )?;
            }
        }
        let roots: Vec<_> = self.atoms.iter().map(|atom| &atom.id).collect();
        walk(&atoms, &roots).map_err(WalkError::diagnostic)?;
        let mut layouts = BTreeSet::new();
        for layout in &self.layouts {
            identity(&layout.id)?;
            if !layouts.insert(&layout.id) {
                return Err(format!("duplicate layout {}", layout.id));
            }
            if layout.sections.is_empty() {
                return Err(format!("layout {}: no sections", layout.id));
            }
            let mut previous_depth = 0;
            let mut placed = BTreeSet::new();
            for (index, section) in layout.sections.iter().enumerate() {
                let at = format!("layout {} section {}", layout.id, index + 1);
                if !section_depth(previous_depth, section.depth) {
                    return Err(format!(
                        "{at}: depth must be 1..6, start at 1 and increase by at most 1"
                    ));
                }
                previous_depth = section.depth;
                let mut titles = BTreeSet::new();
                for title in &section.titles {
                    catalog_admission::variant(&locales, &mut titles, &title.locale).map_err(
                        |error| match error {
                            VariantError::Unregistered => {
                                format!("{at}: unregistered title locale {}", title.locale)
                            }
                            VariantError::Duplicate => {
                                format!("{at}: duplicate title locale {}", title.locale)
                            }
                        },
                    )?;
                    if !single_line(&title.text) {
                        return Err(format!("{at}: title must be nonempty single-line text"));
                    }
                    content_valid(&title.text).map_err(|e| format!("{at}: {e}"))?;
                }
                for id in &section.atoms {
                    if !atoms.contains_key(&id) {
                        return Err(format!("{at}: unknown atom {id}"));
                    }
                    if !placed.insert(id) {
                        return Err(format!("{at}: duplicate placement {id}"));
                    }
                }
            }
        }
        Ok(())
    }
    pub fn file_inputs(&self) -> Vec<&str> {
        self.atoms
            .iter()
            .flat_map(|a| &a.variants)
            .filter_map(|v| match &v.source {
                Content::File { path } => Some(path.as_str()),
                _ => None,
            })
            .collect()
    }
    pub fn compose<'a>(
        &'a self,
        output: &'a Output,
        files: &'a BTreeMap<String, Vec<u8>>,
    ) -> Result<Composition<'a>, String> {
        let locale = self
            .locales
            .iter()
            .find(|l| l.id == output.locale)
            .ok_or_else(|| {
                format!(
                    "output {}: unregistered locale {}",
                    output.id, output.locale
                )
            })?;
        let atoms = self.atoms.iter().map(|atom| (&atom.id, atom)).collect();
        let roots: Vec<_> = output.roots.iter().collect();
        let order = walk(&atoms, &roots)
            .map_err(|error| format!("output {}: {}", output.id, error.diagnostic()))?;
        let mut bodies = Vec::new();
        for atom in order {
            let variant = atom
                .variants
                .iter()
                .find(|v| v.locale == output.locale)
                .ok_or_else(|| {
                    format!(
                        "output {}: atom {} missing selected locale {}",
                        output.id, atom.id, output.locale
                    )
                })?;
            let text = match &variant.source {
                Content::Inline { text } => text.as_str(),
                Content::File { path } => std::str::from_utf8(
                    files
                        .get(path)
                        .ok_or_else(|| format!("missing registered file input {path}"))?,
                )
                .map_err(|e| e.to_string())?,
            };
            content_valid(text).map_err(|e| {
                format!(
                    "output {} / atom {} / locale {}: {e}",
                    output.id, atom.id, output.locale
                )
            })?;
            record_body(&mut bodies, Body { id: &atom.id, text });
        }
        if let Some(id) = &output.layout {
            let at = format!("output {} layout {id}", output.id);
            let layout = layout_for(Some(id), &self.layouts)
                .ok()
                .flatten()
                .map(|index| &self.layouts[index])
                .ok_or_else(|| format!("output {}: unknown layout {id}", output.id))?;
            let mut available = bodies.drain(..).map(|body| (body.id, body)).collect();
            let mut arranged = Vec::new();
            for (index, section) in layout.sections.iter().enumerate() {
                let Some(depth) = rendered_depth(section.depth, output.title.is_some()) else {
                    return Err(format!(
                        "{at} section {}: depth exceeds 6 with output title",
                        index + 1
                    ));
                };
                let title = section
                    .titles
                    .iter()
                    .find(|t| t.locale == output.locale)
                    .ok_or_else(|| {
                        format!(
                            "{at} section {}: missing selected locale {}",
                            index + 1,
                            output.locale
                        )
                    })?;
                arranged.push(format!("{} {}", "#".repeat(depth.into()), title.text));
                let start = bodies.len();
                place_section(&mut available, &mut bodies, &section.atoms).map_err(|atom| {
                    format!("{at}: atom {atom} is outside the selected nonempty body closure")
                })?;
                arranged.extend(bodies[start..].iter().map(|body| body.text.to_owned()));
            }
            if !available.is_empty() {
                let missing = available.keys().map(|id| id.as_str()).collect::<Vec<_>>();
                return Err(format!(
                    "{at}: missing placement for {}",
                    missing.join(", ")
                ));
            }
            return Ok(Composition {
                markdown: arranged.join("\n\n"),
                locale,
                bodies,
            });
        }
        let markdown = bodies
            .iter()
            .map(|body| body.text)
            .collect::<Vec<_>>()
            .join("\n\n");
        Ok(Composition {
            markdown,
            locale,
            bodies,
        })
    }
}

#[cfg(not(verus_keep_ghost))]
pub(super) struct Composition<'a> {
    pub markdown: String,
    pub locale: &'a Locale,
    pub bodies: Vec<Body<'a>>,
}

#[cfg(not(verus_keep_ghost))]
impl WalkError<'_> {
    fn diagnostic(self) -> String {
        match self {
            Self::Cycle(id, path) => {
                let mut chain: Vec<_> = path.into_iter().map(String::as_str).collect();
                chain.push(id.as_str());
                format!("dependency cycle: {}", chain.join(" -> "))
            }
            Self::Missing(id, path) => {
                let chain: Vec<_> = path.into_iter().map(String::as_str).collect();
                format!(
                    "unresolved atom {id}; dependency chain: {}",
                    chain.join(" -> ")
                )
            }
        }
    }
}
#[cfg(not(verus_keep_ghost))]
impl Output {
    fn marker(&self, edge: &str) -> String {
        format!(
            "<!-- chrono-instructions:output producer=chrono-instructions id={} format={} {edge} -->",
            self.id,
            self.format.label()
        )
    }
    pub fn title_body(&self, body: &str) -> String {
        match &self.title {
            Some(title) => format!("# {title}\n\n{body}"),
            None => body.into(),
        }
    }
    pub fn projection(&self, notice: &str, body: &str) -> Vec<u8> {
        format!(
            "{}{}\n{}\n\n{}\n{}\n",
            self.skill.as_ref().map_or(String::new(), Skill::header),
            self.marker("begin"),
            notice,
            self.title_body(body),
            self.marker("end")
        )
        .into_bytes()
    }
    pub fn check_owned(&self, bytes: &[u8]) -> Result<(), String> {
        let failure = || {
            format!(
                "output {} ({}): unowned or malformed projection envelope; preserve the existing file and resolve ownership explicitly",
                self.id, self.path
            )
        };
        let text = std::str::from_utf8(bytes).map_err(|_| failure())?;
        let body = if self.format == Format::Skill {
            // Accept only the producer's scalar frontmatter shape, retaining YAML at byte zero.
            let mut lines = text.splitn(5, '\n');
            if lines.next() != Some("---") {
                return Err(failure());
            }
            let name = lines
                .next()
                .and_then(|s| s.strip_prefix("name: "))
                .ok_or_else(failure)?;
            let description = lines
                .next()
                .and_then(|s| s.strip_prefix("description: "))
                .ok_or_else(failure)?;
            let _: String = serde_json::from_str(name).map_err(|_| failure())?;
            let _: String = serde_json::from_str(description).map_err(|_| failure())?;
            if lines.next() != Some("---") {
                return Err(failure());
            }
            lines.next().ok_or_else(failure)?
        } else {
            text
        };
        if !body.starts_with(&format!("{}\n", self.marker("begin")))
            || !body.ends_with(&format!("\n{}\n", self.marker("end")))
            || text.matches("<!-- chrono-instructions").count() != 2
        {
            return Err(failure());
        }
        Ok(())
    }
}
