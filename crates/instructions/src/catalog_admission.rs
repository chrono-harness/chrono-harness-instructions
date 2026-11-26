//! Admission before a manifest or catalog can publish host guides.
use std::collections::{BTreeMap, BTreeSet};
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;

verus! {

pub enum Registration<M, I> {
    Retained(M),
    Initialize(I),
    Missing,
}

pub fn registration<M, I>(manifest: Option<M>, init: Option<I>) -> (source: Registration<M, I>)
    ensures match manifest {
        Some(saved) => source == Registration::<M, I>::Retained(saved),
        None => match init {
            Some(selected) => source == Registration::<M, I>::Initialize(selected),
            None => source is Missing,
        },
    },
{
    match manifest {
        Some(saved) => Registration::Retained(saved),
        None => match init {
            Some(selected) => Registration::Initialize(selected),
            None => Registration::Missing,
        },
    }
}

/// An omitted init argument inherits its registered source. An explicit one binds exact bytes.
pub fn accepts_override(selected: Option<&[u8]>, registered: Option<&[u8]>) -> (accept: bool)
    ensures accept == match selected {
        None => true,
        Some(bytes) => match registered { Some(saved) => bytes@ == saved@, None => false },
    },
{
    match (selected, registered) {
        (None, _) => true,
        (Some(bytes), Some(saved)) => super::verified_bytes::equal(bytes, saved),
        (Some(_), None) => false,
    }
}

pub enum PathRole { Control, Source, Output }

/// File variants sharing a source have already been grouped by their path.
pub fn claim_path<K: Ord>(names: &mut BTreeSet<K>, path: K, role: PathRole) -> (out: Result<(), &'static str>)
    requires vstd::laws_cmp::obeys_cmp::<K>(),
    ensures names@ == old(names)@.insert(path),
        out == (if !old(names)@.contains(path) { Ok(()) } else { Err(match role {
            PathRole::Control => "source/control/alias overlap",
            PathRole::Source => "source/control overlap",
            PathRole::Output => "source/output/alias overlap",
        }) }),
{
    if names.insert(path) { return Ok(()); }
    Err(match role {
        PathRole::Control => "source/control/alias overlap",
        PathRole::Source => "source/control overlap",
        PathRole::Output => "source/output/alias overlap",
    })
}

/// Folded directory prefixes may repeat with identical spelling. File identities are exclusive.
pub fn alias_conflict<'a>(prior: Option<&'a String>, path: &String, shared_prefix: bool) -> (conflict: Option<&'a String>)
    ensures conflict == match prior {
        Some(old) if !shared_prefix || old@ != path@ => Some(old),
        _ => None,
    },
{
    match prior {
        Some(old) if !shared_prefix || !String::eq(old, path) => Some(old),
        _ => None,
    }
}

pub fn descendant_path(parent: &[u8], other: &[u8]) -> (found: bool)
    ensures found == (other.len() > parent.len() && other@[parent.len() as int] == 47u8
        && other@.subrange(0,parent.len() as int) == parent@),
{
    other.len() > parent.len() && other[parent.len()] == 47u8 &&
        super::verified_bytes::equal(parent, &other[0..parent.len()])
}

pub enum OutputError { DuplicateId, DuplicatePath, EmptyRoots }

pub fn output<K: Ord>(
    ids: &mut BTreeSet<K>, paths: &mut BTreeSet<K>, id: K, path: K, roots: usize,
) -> (out: Result<(), OutputError>)
    requires vstd::laws_cmp::obeys_cmp::<K>(),
    ensures ids@ == old(ids)@.insert(id),
        paths@ == if old(ids)@.contains(id) { old(paths)@ } else { old(paths)@.insert(path) },
        match out {
            Err(OutputError::DuplicateId) => old(ids)@.contains(id),
            Err(OutputError::DuplicatePath) => !old(ids)@.contains(id) && old(paths)@.contains(path),
            Err(OutputError::EmptyRoots) => !old(ids)@.contains(id) && !old(paths)@.contains(path) && roots == 0,
            Ok(()) => !old(ids)@.contains(id) && !old(paths)@.contains(path) && roots > 0,
        },
{
    if !ids.insert(id) { return Err(OutputError::DuplicateId); }
    if !paths.insert(path) { return Err(OutputError::DuplicatePath); }
    if roots == 0 { return Err(OutputError::EmptyRoots); }
    Ok(())
}

pub fn root_output(root: bool, path: &[u8]) -> (out: Result<bool, &'static str>)
    ensures match out {
        Ok(is_root) => is_root == root && (path@ == "CLAUDE.md".spec_bytes()) == root,
        Err(message) => if root { path@ != "CLAUDE.md".spec_bytes() && message@ == "root-guide path must be CLAUDE.md"@ }
            else { path@ == "CLAUDE.md".spec_bytes() && message@ == "CLAUDE.md is reserved for root-guide"@ },
    },
{
    if super::verified_bytes::equal(path, "CLAUDE.md".as_bytes()) {
        if root { Ok(true) } else { Err("CLAUDE.md is reserved for root-guide") }
    } else if root { Err("root-guide path must be CLAUDE.md") } else { Ok(false) }
}

pub fn finish_roots(count: usize) -> (out: Result<(), &'static str>)
    ensures match out { Ok(()) => count == 1, Err(msg) => count != 1 && msg@ == "registration must contain exactly one root-guide"@ },
{
    if count == 1 { Ok(()) } else { Err("registration must contain exactly one root-guide") }
}

pub enum LocaleError { Duplicate, EmptyFrame }

pub fn locale<K: Ord>(locales: &mut BTreeSet<K>, id: K, frame_empty: bool, notice_empty: bool)
    -> (out: Result<(), LocaleError>)
    requires vstd::laws_cmp::obeys_cmp::<K>(),
    ensures locales@ == old(locales)@.insert(id),
        match out {
            Err(LocaleError::Duplicate) => old(locales)@.contains(id),
            Err(LocaleError::EmptyFrame) => !old(locales)@.contains(id) && (frame_empty || notice_empty),
            Ok(()) => !old(locales)@.contains(id) && !frame_empty && !notice_empty,
        },
{
    if !locales.insert(id) { return Err(LocaleError::Duplicate); }
    if frame_empty || notice_empty { return Err(LocaleError::EmptyFrame); }
    Ok(())
}

pub fn register_atom<K: Ord, V>(atoms: &mut BTreeMap<K,V>, id: K, atom: V) -> (fresh: bool)
    requires vstd::laws_cmp::obeys_cmp::<K>(),
    ensures atoms@ == old(atoms)@.insert(id,atom), fresh == !old(atoms)@.contains_key(id),
{
    atoms.insert(id,atom).is_none()
}

pub enum VariantError { Unregistered, Duplicate }
pub fn variant<K: Ord>(locales: &BTreeSet<K>, variants: &mut BTreeSet<K>, locale: K)
    -> (out: Result<(), VariantError>)
    requires vstd::laws_cmp::obeys_cmp::<K>(),
    ensures variants@ == if locales@.contains(locale) { old(variants)@.insert(locale) } else { old(variants)@ },
        match out {
            Err(VariantError::Unregistered) => !locales@.contains(locale),
            Err(VariantError::Duplicate) => locales@.contains(locale) && old(variants)@.contains(locale),
            Ok(()) => locales@.contains(locale) && !old(variants)@.contains(locale),
        },
{
    broadcast use vstd::std_specs::btree::axiom_set_contains_deref_key;
    if !locales.contains(&locale) { return Err(VariantError::Unregistered); }
    if !variants.insert(locale) { return Err(VariantError::Duplicate); }
    Ok(())
}

}
