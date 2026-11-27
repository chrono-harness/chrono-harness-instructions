#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub const LF: u8 = 10;
pub const HASH: u8 = 35;
pub const SPACE: u8 = 32;

/// Append one published rule with its untouched body, writing the notice on the first row.
pub fn append_rule(result: &mut Vec<u8>, notice: &[u8], id: &[u8], text: &[u8])
    ensures result@ ==
        (if old(result)@.len() == 0 { notice@ + seq![LF] } else { old(result)@ })
        + seq![LF, HASH, HASH, SPACE] + id@ + seq![LF, LF] + text@ + seq![LF],
{
    broadcast use vstd::array::group_array_axioms;
    if result.is_empty() {
        result.extend_from_slice(notice);
        result.push(LF);
    }
    result.extend_from_slice(&[LF, HASH, HASH, SPACE]);
    result.extend_from_slice(id);
    result.extend_from_slice(&[LF, LF]);
    result.extend_from_slice(text);
    result.push(LF);
}
}

#[cfg(not(verus_keep_ghost))]
use super::catalog::{self, Catalog, Content, Manifest};
#[cfg(not(verus_keep_ghost))]
use crate::rule_selection::{self, Mode, Row};
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize};
#[cfg(not(verus_keep_ghost))]
use std::{collections::BTreeSet, fs, io, path::Path, process::Command};

#[cfg(not(verus_keep_ghost))]
const CHANNEL: &str = "https://raw.githubusercontent.com/chrono-harness/chrono-harness-instructions/published-rules/rules.json";
#[cfg(not(verus_keep_ghost))]
const SCHEMA: &str = "chrono-agent-rules/v1";
#[cfg(not(verus_keep_ghost))]
const CONFIG: &str = ".chrono-harness/rules.json";

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Feed {
    schema: String,
    notice: String,
    rules: Vec<Rule>,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Rule {
    id: String,
    text: String,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
#[serde(tag = "mode", rename_all = "lowercase", deny_unknown_fields)]
enum Selection {
    All {
        #[serde(default)]
        exclude: BTreeSet<String>,
    },
    Only {
        include: BTreeSet<String>,
    },
    Off,
}

#[cfg(not(verus_keep_ghost))]
impl Selection {
    fn read(root: &Path) -> Result<Self, String> {
        if !root.is_dir() {
            return Err(format!("host root is not a directory: {}", root.display()));
        }
        let path = root.join(CONFIG);
        match fs::read(&path) {
            Ok(bytes) => {
                serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", path.display()))
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Self::All {
                exclude: BTreeSet::new(),
            }),
            Err(e) => Err(format!("{}: {e}", path.display())),
        }
    }

    fn policy(&self) -> (Mode, Option<&BTreeSet<String>>) {
        match self {
            Self::All { exclude } => (Mode::All, Some(exclude)),
            Self::Only { include } => (Mode::Only, Some(include)),
            Self::Off => (Mode::Off, None),
        }
    }
}

#[cfg(not(verus_keep_ghost))]
pub(super) fn publish() -> Result<String, String> {
    let manifest: Manifest = catalog::parse(catalog::DEFAULT_MANIFEST.as_bytes())?;
    let catalog: Catalog = catalog::parse(catalog::DEFAULT_CATALOG.as_bytes())?;
    manifest.validate()?;
    catalog.validate()?;
    let output = manifest.root();
    // Compose validates the full closure and the layout against its nonempty bodies.
    let (_, locale) = catalog.compose(output, &Default::default())?;
    let ids = catalog.ordered_rule_ids(output)?;
    let mut rules = Vec::new();
    for id in ids {
        let atom = catalog
            .atoms
            .iter()
            .find(|a| a.id == id)
            .ok_or_else(|| format!("unknown rule {id}"))?;
        let variant = atom
            .variants
            .iter()
            .find(|v| v.locale == output.locale)
            .ok_or_else(|| format!("missing locale for {id}"))?;
        let Content::Inline { text } = &variant.source else {
            return Err(format!("published rule {id} requires an unbundled input"));
        };
        if !text.is_empty() {
            rules.push(Rule {
                id: id.to_owned(),
                text: text.clone(),
            });
        }
    }
    let feed = Feed {
        schema: SCHEMA.into(),
        notice: locale.projection_notice.clone(),
        rules,
    };
    serde_json::to_string_pretty(&feed)
        .map(|s| s + "\n")
        .map_err(|e| e.to_string())
}

#[cfg(not(verus_keep_ghost))]
pub(super) fn read(
    root: &Path,
    fetch: impl FnOnce() -> Result<Vec<u8>, String>,
) -> Result<String, String> {
    read_selected(Selection::read(root)?, fetch)
}

#[cfg(not(verus_keep_ghost))]
fn read_selected(
    selection: Selection,
    fetch: impl FnOnce() -> Result<Vec<u8>, String>,
) -> Result<String, String> {
    let (mode, names) = selection.policy();
    if !rule_selection::needs_feed(&mode, names.map_or(0, BTreeSet::len)) {
        return Ok(String::new());
    }
    let feed: Feed =
        serde_json::from_slice(&fetch()?).map_err(|e| format!("invalid upstream rules: {e}"))?;
    if !rule_selection::schema(feed.schema.as_bytes(), SCHEMA.as_bytes()) {
        return Err(format!("unsupported rules schema: {}", feed.schema));
    }
    let mut ids = BTreeSet::new();
    let mut result = Vec::new();
    for rule in feed.rules {
        let previous = ids.replace(rule.id.clone());
        let listed = names.and_then(|names| names.get(&rule.id));
        match rule_selection::row(
            &mode,
            rule.id.as_bytes(),
            rule.text.trim().as_bytes(),
            listed.map(String::as_bytes),
            previous.as_ref().map(String::as_bytes),
        ) {
            Row::Invalid => return Err(format!("empty or duplicate upstream rule: {}", rule.id)),
            Row::Omit => (),
            Row::Publish => append_rule(
                &mut result,
                feed.notice.as_bytes(),
                rule.id.as_bytes(),
                rule.text.as_bytes(),
            ),
        }
    }
    String::from_utf8(result).map_err(|error| error.to_string())
}

/// Publish a materialized guide while preserving host text and the literal root alias.
#[cfg(not(verus_keep_ghost))]
pub(super) fn sync(
    host: &Path,
    fetch: impl FnOnce() -> Result<Vec<u8>, String>,
) -> Result<Vec<std::path::PathBuf>, String> {
    use super::{BEGIN, END, Fault, MANIFEST, RESERVED, metadata, regular, transaction};
    use std::collections::BTreeMap;

    transaction::run(host, Fault::default(), |root| {
        super::parents(root, MANIFEST, &mut BTreeSet::new())?;
        if metadata(&root.join(MANIFEST))?.is_some() {
            return Err("host uses a local instruction manifest; use generate".into());
        }
        let selected = regular(&root.join(CONFIG))?;
        let selection = match &selected {
            Some(input) => serde_json::from_slice(&input.bytes)
                .map_err(|error| format!("{CONFIG}: {error}"))?,
            None => Selection::All {
                exclude: BTreeSet::new(),
            },
        };
        let text = read_selected(selection, fetch)?;
        if !rule_selection::safe_projection(text.as_bytes(), RESERVED) {
            return Err("upstream rules contain reserved guide delimiters".into());
        }
        let mut block = BEGIN.to_vec();
        block.push(LF);
        block.extend_from_slice(text.as_bytes());
        block.extend_from_slice(END);
        let mut changes = vec![];
        let alias = super::root_guides(root, &block, &mut changes)?;
        let mut observed = BTreeMap::new();
        if let Some(snapshot) = &selected {
            observed.insert(CONFIG.to_owned(), transaction::sha256(&snapshot.bytes));
        }
        let mut bindings = transaction::bind_inputs(
            root,
            observed,
            changes.iter().filter(|change| change.unchanged()).collect(),
            alias,
        )?;
        if selected.is_none() {
            bindings.require_absent(CONFIG);
        }
        bindings.require_absent(MANIFEST);
        changes.retain(|change| !change.unchanged());
        Ok((changes, vec![], bindings))
    })
}

#[cfg(not(verus_keep_ghost))]
pub(super) fn fetch() -> Result<Vec<u8>, String> {
    let output = Command::new("curl")
        .args([
            "-q",
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--proto",
            "=https",
            "--proto-redir",
            "=https",
            "--max-time",
            "30",
            "--max-filesize",
            "4194304",
            CHANNEL,
        ])
        .output()
        .map_err(|e| format!("fetch upstream rules: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "fetch upstream rules ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(output.stdout)
}
