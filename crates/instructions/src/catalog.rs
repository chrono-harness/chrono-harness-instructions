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

pub(crate) fn active_dependency(stack: &[String], id: &str) -> (active: bool)
    ensures active == (exists|i: int| 0 <= i < stack@.len()
        && encode_utf8(stack@[i]@) == encode_utf8(id@)),
{
    let mut i = 0;
    while i < stack.len()
        invariant i <= stack@.len(),
            forall|j: int| 0 <= j < i ==> encode_utf8(stack@[j]@) != encode_utf8(id@),
        decreases stack.len() - i,
    {
        if super::verified_bytes::equal(stack[i].as_str().as_bytes(), id.as_bytes()) {
            return true;
        }
        i += 1;
    }
    false
}

pub(crate) enum Dependency<T> {
    Cycle,
    Visited,
    Missing,
    Open(T),
}

pub(crate) fn dependency<T>(stack: &[String], id: &str, visited: bool, entry: Option<T>)
    -> (step: Dependency<T>)
    ensures match step {
        Dependency::Cycle => exists|i: int| 0 <= i < stack@.len()
            && encode_utf8(stack@[i]@) == encode_utf8(id@),
        Dependency::Visited => visited && forall|i: int| 0 <= i < stack@.len()
            ==> encode_utf8(stack@[i]@) != encode_utf8(id@),
        Dependency::Missing => !visited && entry.is_none() && forall|i: int| 0 <= i < stack@.len()
            ==> encode_utf8(stack@[i]@) != encode_utf8(id@),
        Dependency::Open(atom) => !visited && entry == Some(atom) && forall|i: int| 0 <= i < stack@.len()
            ==> encode_utf8(stack@[i]@) != encode_utf8(id@),
    },
{
    if active_dependency(stack, id) {
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

/// Without a layout the dependency traversal already supplies the publication order.
pub(crate) fn selected_ids<'a>(atoms: &[&'a Atom]) -> (ids: Vec<&'a str>)
    ensures ids@.len() == atoms@.len(),
        forall|i:int| 0 <= i < atoms@.len() ==> ids@[i]@ == atoms@[i].id@,
{
    let mut ids: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < atoms.len()
        invariant i <= atoms@.len(), ids@.len() == i,
            forall|j:int| 0 <= j < i ==> ids@[j]@ == atoms@[j].id@,
        decreases atoms.len() - i,
    {
        ids.push(atoms[i].id.as_str());
        i += 1;
    }
    ids
}

pub(crate) open spec fn section_order(sections: Seq<Section>) -> Seq<Seq<char>>
    decreases sections.len(),
{
    if sections.len() == 0 { Seq::empty() }
    else { section_order(sections.drop_last()) + sections.last().atoms@.map(|_index, id: String| id@) }
}

/// Append the actual sections in declaration order, including empty sections.
pub(crate) fn layout_ids(sections: &[Section]) -> (ids: Vec<&str>)
    ensures ids@.map(|_index, id: &str| id@) == section_order(sections@),
{
    let mut ids: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < sections.len()
        invariant i <= sections@.len(),
            ids@.map(|_index, id: &str| id@) == section_order(sections@.take(i as int)),
        decreases sections.len() - i,
    {
        let ghost previous = ids@;
        append_section(&mut ids, &sections[i].atoms);
        proof {
            let prefix = sections@.take(i + 1);
            assert(prefix.drop_last() =~= sections@.take(i as int));
            assert(prefix.last() == sections@[i as int]);
            assert(ids@.map(|_index, id: &str| id@) =~=
                previous.map(|_index, id: &str| id@) + sections@[i as int].atoms@.map(|_index, id: String| id@));
        }
        i += 1;
    }
    assert(sections@.take(i as int) =~= sections@);
    ids
}

/// A section starts after the entire preceding prefix and retains each declared ID in order.
pub(crate) fn append_section<'a>(order: &mut Vec<&'a str>, atoms: &'a [String])
    ensures order@.len() == old(order)@.len() + atoms@.len(),
        forall|i: int| 0 <= i < old(order)@.len() ==> order@[i] == old(order)@[i],
        forall|i: int| 0 <= i < atoms@.len() ==> order@[old(order)@.len() + i]@ == atoms@[i]@,
{
    let mut i = 0;
    while i < atoms.len()
        invariant i <= atoms@.len(), order@.len() == old(order)@.len() + i,
            forall|j: int| 0 <= j < old(order)@.len() ==> order@[j] == old(order)@[j],
            forall|j: int| 0 <= j < i ==> order@[old(order)@.len() + j]@ == atoms@[j]@,
        decreases atoms.len() - i,
    {
        order.push(atoms[i].as_str());
        i += 1;
    }
}
}

#[cfg(not(verus_keep_ghost))]
use crate::rule_selection::catalog_admission::{self, LocaleError, OutputError, VariantError};
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize};
#[cfg(not(verus_keep_ghost))]
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
            if !catalog_admission::register_atom(&mut atoms, atom.id.as_str(), atom) {
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
        let mut visited = BTreeSet::new();
        for atom in &self.atoms {
            visit(
                &atom.id,
                &atoms,
                &mut visited,
                &mut Vec::new(),
                &mut Vec::new(),
            )?;
        }
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
                    if !atoms.contains_key(id.as_str()) {
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
    fn selected_atoms(&self, output: &Output) -> Result<Vec<&Atom>, String> {
        let atoms: BTreeMap<_, _> = self.atoms.iter().map(|a| (a.id.as_str(), a)).collect();
        let mut order = Vec::new();
        let mut visited = BTreeSet::new();
        for root in &output.roots {
            visit(root, &atoms, &mut visited, &mut Vec::new(), &mut order)
                .map_err(|e| format!("output {}: {e}", output.id))?;
        }
        Ok(order)
    }

    pub fn ordered_rule_ids(&self, output: &Output) -> Result<Vec<&str>, String> {
        if let Some(layout) = layout_for(output.layout.as_ref(), &self.layouts)
            .map_err(|id| format!("unknown layout {id}"))?
        {
            return Ok(layout_ids(&self.layouts[layout].sections));
        }
        Ok(selected_ids(&self.selected_atoms(output)?))
    }

    pub fn compose(
        &self,
        output: &Output,
        files: &BTreeMap<String, Vec<u8>>,
    ) -> Result<(String, &Locale), String> {
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
        let order = self.selected_atoms(output)?;
        let mut parts = Vec::new();
        let mut bodies = BTreeMap::new();
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
            if !text.is_empty() {
                parts.push(text);
                bodies.insert(atom.id.as_str(), text);
            }
        }
        if let Some(id) = &output.layout {
            let at = format!("output {} layout {id}", output.id);
            let layout = layout_for(Some(id), &self.layouts)
                .ok()
                .flatten()
                .map(|index| &self.layouts[index])
                .ok_or_else(|| format!("output {}: unknown layout {id}", output.id))?;
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
                for atom in &section.atoms {
                    let text = bodies.remove(atom.as_str()).ok_or_else(|| {
                        format!("{at}: atom {atom} is outside the selected nonempty body closure")
                    })?;
                    arranged.push(text.to_owned());
                }
            }
            if !bodies.is_empty() {
                return Err(format!(
                    "{at}: missing placement for {}",
                    bodies.keys().copied().collect::<Vec<_>>().join(", ")
                ));
            }
            return Ok((arranged.join("\n\n"), locale));
        }
        Ok((parts.join("\n\n"), locale))
    }
}
#[cfg(not(verus_keep_ghost))]
fn visit<'a>(
    id: &str,
    atoms: &BTreeMap<&str, &'a Atom>,
    visited: &mut BTreeSet<String>,
    stack: &mut Vec<String>,
    order: &mut Vec<&'a Atom>,
) -> Result<(), String> {
    let atom = match dependency(stack, id, visited.contains(id), atoms.get(id).copied()) {
        Dependency::Cycle => {
            let mut chain = stack.clone();
            chain.push(id.into());
            return Err(format!("dependency cycle: {}", chain.join(" -> ")));
        }
        Dependency::Visited => return Ok(()),
        Dependency::Missing => {
            return Err(format!(
                "unresolved atom {id}; dependency chain: {}",
                stack.join(" -> ")
            ));
        }
        Dependency::Open(atom) => atom,
    };
    stack.push(id.into());
    for required in &atom.requires {
        visit(required, atoms, visited, stack, order)?;
    }
    stack.pop();
    visited.insert(id.into());
    order.push(atom);
    Ok(())
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
