use super::catalog::{self, Catalog, Content, Manifest};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs, io, path::Path, process::Command};

const CHANNEL: &str = "https://raw.githubusercontent.com/chrono-harness/chrono-harness-instructions/published-rules/rules.json";
const SCHEMA: &str = "chrono-agent-rules/v1";
const CONFIG: &str = ".chrono-harness/rules.json";

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Feed {
    schema: String,
    notice: String,
    rules: Vec<Rule>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Rule {
    id: String,
    text: String,
}

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

    fn accepts(&self, id: &str) -> bool {
        match self {
            Self::All { exclude } => !exclude.contains(id),
            Self::Only { include } => include.contains(id),
            Self::Off => false,
        }
    }
}

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

pub(super) fn read(
    root: &Path,
    fetch: impl FnOnce() -> Result<Vec<u8>, String>,
) -> Result<String, String> {
    let selection = Selection::read(root)?;
    if matches!(&selection, Selection::Off)
        || matches!(&selection, Selection::Only { include } if include.is_empty())
    {
        return Ok(String::new());
    }
    let feed: Feed =
        serde_json::from_slice(&fetch()?).map_err(|e| format!("invalid upstream rules: {e}"))?;
    if feed.schema != SCHEMA {
        return Err(format!("unsupported rules schema: {}", feed.schema));
    }
    let mut ids = BTreeSet::new();
    let mut result = String::new();
    for rule in feed.rules {
        if rule.id.is_empty() || rule.text.trim().is_empty() || !ids.insert(rule.id.clone()) {
            return Err(format!("empty or duplicate upstream rule: {}", rule.id));
        }
        if selection.accepts(&rule.id) {
            if result.is_empty() {
                result = format!("{}\n", feed.notice);
            }
            result.push_str(&format!("\n## {}\n\n{}\n", rule.id, rule.text));
        }
    }
    Ok(result)
}

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
