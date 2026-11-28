//! Explicit host instruction generation. No judge execution or host discovery.

#[cfg(verus_keep_ghost)]
use super::verified_bytes;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::{prelude::*, string::StringSliceAdditionalSpecFns};

verus! {
pub(crate) enum PathUse { Allowed, NonCanonical, Reserved, ExternalSource }

/// A registered relative path has only nonempty, named components.
pub(crate) fn canonical_component(part: &[u8]) -> (valid: bool)
    ensures valid == (part.len() > 0
        && !(part.len() == 1 && part@[0] == 46u8)
        && !(part.len() == 2 && part@[0] == 46u8 && part@[1] == 46u8)),
{
    match part.len() {
        0 => false,
        1 => part[0] != 46u8,
        2 => part[0] != 46u8 || part[1] != 46u8,
        _ => true,
    }
}

pub open spec fn under(path: Seq<u8>, directory: Seq<u8>) -> bool {
    path.len() > directory.len()
        && path.subrange(0, directory.len() as int) == directory
        && path[directory.len() as int] == 47u8
}

/// Distinguish a registered directory, its descendants and unrelated prefix matches.
pub(crate) fn directory_relation(path: &[u8], directory: &[u8]) -> (relation: (bool, bool))
    ensures relation.0 == (path@ == directory@), relation.1 == under(path@, directory@),
{
    if path.len() < directory.len() { return (false,false); }
    let same = verified_bytes::equal(&path[0..directory.len()], directory);
    if path.len() == directory.len() {
        assert(path@.subrange(0, directory.len() as int) =~= path@);
        (same, false)
    } else {
        (false, same && path[directory.len()] == 47u8)
    }
}

pub(crate) fn registered_path(path: &[u8], canonical: bool, source: bool) -> (result: PathUse)
    ensures ({
        let state = ".chrono-harness/state".spec_bytes();
        let retention = ".chrono-harness/retention.json".spec_bytes();
        let reserved = path@ == state || under(path@, state)
            || path@ == retention || under(path@, retention);
        let external = source && !under(path@, ".chrono-harness".spec_bytes());
        &&& (result is NonCanonical) == !canonical
        &&& (result is Reserved) == (canonical && reserved)
        &&& (result is ExternalSource) == (canonical && !reserved && external)
        &&& (result is Allowed) == (canonical && !reserved && !external)
    }),
{
    if !canonical { return PathUse::NonCanonical; }
    let state = directory_relation(path, ".chrono-harness/state".as_bytes());
    let retention = directory_relation(path, ".chrono-harness/retention.json".as_bytes());
    if state.0 || state.1 || retention.0 || retention.1 { return PathUse::Reserved; }
    if source && !directory_relation(path, ".chrono-harness".as_bytes()).1 {
        return PathUse::ExternalSource;
    }
    PathUse::Allowed
}
}

#[cfg(not(verus_keep_ghost))]
mod catalog;
#[cfg(not(verus_keep_ghost))]
pub mod event_identity;
#[cfg(not(verus_keep_ghost))]
pub mod github_event;
#[cfg(not(verus_keep_ghost))]
mod inherit;
#[doc(hidden)]
#[cfg(not(verus_keep_ghost))]
pub mod publication_decision;
#[cfg(not(verus_keep_ghost))]
pub mod rule_selection;
#[cfg(not(verus_keep_ghost))]
mod transaction;
#[cfg(not(verus_keep_ghost))]
mod verified_bytes;

#[cfg(not(verus_keep_ghost))]
use catalog::{Atom, Catalog, Content, Format, Manifest, Variant};
#[cfg(not(verus_keep_ghost))]
use std::collections::{BTreeMap, BTreeSet};

#[cfg(not(verus_keep_ghost))]
use std::ffi::{OsStr, OsString};
#[cfg(not(verus_keep_ghost))]
use std::fs;
#[cfg(not(verus_keep_ghost))]
use std::path::{Path, PathBuf};
#[cfg(not(verus_keep_ghost))]
use transaction::{Change, Fault, Snapshot};

#[cfg(not(verus_keep_ghost))]
const MANIFEST: &str = ".chrono-harness/instructions/manifest.json";
#[cfg(not(verus_keep_ghost))]
const METHOD: &str = ".chrono-harness/instructions/methodology.md";
#[cfg(not(verus_keep_ghost))]
const CONTEXT: &str = ".chrono-harness/instructions/host-context.md";
#[cfg(not(verus_keep_ghost))]
const BEGIN: &[u8] = b"<!-- chrono-instructions:begin -->";
#[cfg(not(verus_keep_ghost))]
const END: &[u8] = b"<!-- chrono-instructions:end -->";
#[cfg(not(verus_keep_ghost))]
const RESERVED: &[u8] = b"<!-- chrono-instructions";
#[cfg(not(verus_keep_ghost))]
const CATALOG: &str = ".chrono-harness/instructions/catalog.json";

#[cfg(not(verus_keep_ghost))]
struct InitSources {
    methodology: Option<Vec<u8>>,
    context: Option<Vec<u8>>,
    locale: Option<String>,
    external: Vec<transaction::ExternalInput>,
}

#[cfg(not(verus_keep_ghost))]
impl InitSources {
    fn read(methodology: Option<&Path>, context: Option<&Path>) -> Result<Self, String> {
        let mut external = Vec::new();
        let mut read = |path: &Path| {
            let (bytes, binding) = transaction::external_input(path)?;
            utf8(path, &bytes)?;
            external.push(binding);
            Ok::<_, String>(bytes)
        };
        let methodology = methodology.map(&mut read).transpose()?;
        let context = context.map(&mut read).transpose()?;
        Ok(Self {
            methodology,
            context,
            locale: None,
            external,
        })
    }
}

#[cfg(not(verus_keep_ghost))]
fn managed_block(manifest: &Manifest, frame: &str, body: &str) -> Vec<u8> {
    let mut block = BEGIN.to_vec();
    block.extend_from_slice(
        format!(
            "\n{frame}\n\nmanifest: `{MANIFEST}`\ncatalog: `{}`\nhost_context: `{}`\n\n{}\n",
            manifest.catalog,
            manifest.host_context,
            manifest.root().title_body(body)
        )
        .as_bytes(),
    );
    block.extend_from_slice(END);
    block
}

#[cfg(not(verus_keep_ghost))]
fn error(path: &Path, message: impl std::fmt::Display) -> String {
    format!("{}: {message}", path.display())
}

#[cfg(not(verus_keep_ghost))]
fn metadata(path: &Path) -> Result<Option<fs::Metadata>, String> {
    match fs::symlink_metadata(path) {
        Ok(meta) => Ok(Some(meta)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(error(path, e)),
    }
}

#[cfg(not(verus_keep_ghost))]
fn regular(path: &Path) -> Result<Option<Snapshot>, String> {
    let Some(meta) = metadata(path)? else {
        return Ok(None);
    };
    if !meta.file_type().is_file() {
        return Err(error(
            path,
            "expected a regular file (aliases are not supported here)",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if meta.nlink() != 1 {
            return Err(error(
                path,
                "hard-linked retained files/outputs are unsupported",
            ));
        }
    }
    let identity = transaction::identity(path)?;
    let bytes = fs::read(path).map_err(|e| error(path, e))?;
    let after = fs::symlink_metadata(path).map_err(|e| error(path, e))?;
    use std::os::unix::fs::MetadataExt;
    if transaction::identity(path)? != identity
        || (
            meta.len(),
            meta.mode(),
            meta.mtime(),
            meta.mtime_nsec(),
            meta.ctime(),
            meta.ctime_nsec(),
            meta.nlink(),
        ) != (
            after.len(),
            after.mode(),
            after.mtime(),
            after.mtime_nsec(),
            after.ctime(),
            after.ctime_nsec(),
            after.nlink(),
        )
    {
        return Err(error(
            path,
            "regular file identity/content changed during read",
        ));
    }
    Ok(Some(Snapshot {
        bytes,
        permissions: meta.permissions(),
        identity,
    }))
}

#[cfg(not(verus_keep_ghost))]
fn utf8(path: &Path, bytes: &[u8]) -> Result<(), String> {
    std::str::from_utf8(bytes)
        .map(|_| ())
        .map_err(|e| error(path, format!("expected UTF-8: {e}")))
}

#[cfg(not(verus_keep_ghost))]
fn positions(bytes: &[u8], needle: &[u8]) -> Vec<usize> {
    bytes
        .windows(needle.len())
        .enumerate()
        .filter_map(|(i, part)| (part == needle).then_some(i))
        .collect()
}

#[cfg(not(verus_keep_ghost))]
fn render(path: &Path, bytes: &[u8], block: &[u8]) -> Result<Vec<u8>, String> {
    use rule_selection::GuideEdit;
    let reserved = positions(bytes, RESERVED);
    let (starts, ends) = if reserved.is_empty() {
        (Vec::new(), Vec::new())
    } else {
        (positions(bytes, BEGIN), positions(bytes, END))
    };
    match rule_selection::guide_edit(
        bytes,
        reserved.len(),
        &starts,
        &ends,
        BEGIN.len(),
        END.len(),
    ) {
        GuideEdit::Append(separator) => {
            let mut result = bytes.to_vec();
            if separator {
                result.push(b'\n');
            }
            result.extend_from_slice(block);
            result.push(b'\n');
            Ok(result)
        }
        GuideEdit::Replace(start, end) => {
            let mut result = bytes[..start].to_vec();
            result.extend_from_slice(block);
            result.extend_from_slice(&bytes[end..]);
            Ok(result)
        }
        GuideEdit::Malformed => Err(error(path, "malformed or duplicate managed delimiters")),
        GuideEdit::InvalidLine => Err(error(
            path,
            "managed delimiters must be ordered standalone lines",
        )),
    }
}

// Canonical lexical paths keep the registry unambiguous; existing ancestors are
// checked separately before reading or writing any registered file.
#[cfg(not(verus_keep_ghost))]
fn relative(path: &str, source: bool) -> Result<(), String> {
    catalog::content_valid(path)?;
    let canonical = !Path::new(path).is_absolute()
        && !path.contains('\\')
        && !path.chars().any(char::is_control)
        && path
            .split('/')
            .all(|part| canonical_component(part.as_bytes()));
    match registered_path(path.as_bytes(), canonical, source) {
        PathUse::Allowed => Ok(()),
        PathUse::NonCanonical => Err(format!("{path:?}: expected canonical relative path")),
        PathUse::Reserved => Err(format!("{path}: reserved instruction state or policy path")),
        PathUse::ExternalSource => Err(format!(
            "{path}: registered inputs must be under .chrono-harness/"
        )),
    }
}

// Use Unicode case folding, then the platform's filesystem representation for
// prospective macOS paths. Lowercasing misses aliases such as straße / STRASSE.
// This conservatively rejects ambiguous spellings even on case-sensitive volumes;
// it does not change registered paths or probe the filesystem with writes.
#[cfg(target_os = "macos")]
#[cfg(not(verus_keep_ghost))]
fn mac_path_key(path: &str) -> Result<Vec<u8>, String> {
    use std::ffi::{c_char, c_void};
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFStringCreateWithBytes(
            allocator: *const c_void,
            bytes: *const u8,
            count: isize,
            encoding: u32,
            external: u8,
        ) -> *const c_void;
        fn CFStringCreateMutableCopy(
            allocator: *const c_void,
            capacity: isize,
            string: *const c_void,
        ) -> *mut c_void;
        fn CFStringFold(string: *mut c_void, flags: usize, locale: *const c_void);
        fn CFStringGetMaximumSizeOfFileSystemRepresentation(string: *const c_void) -> isize;
        fn CFStringGetFileSystemRepresentation(
            string: *const c_void,
            buffer: *mut c_char,
            size: isize,
        ) -> u8;
        fn CFRelease(value: *const c_void);
    }
    let count = isize::try_from(path.len()).map_err(|_| format!("{path}: path too long"))?;
    // SAFETY: create from a live UTF-8 slice using the SDK's UTF-8 encoding
    // constant. Fold only a mutable copy, using kCFCompareCaseInsensitive (1)
    // and the canonical system locale (NULL), not the user's locale. Check the
    // allocations and buffer bound; release both owned CF strings on every
    // return path. CF never retains the supplied byte buffer.
    unsafe {
        let original =
            CFStringCreateWithBytes(std::ptr::null(), path.as_ptr(), count, 0x08000100, 0);
        if original.is_null() {
            return Err(format!(
                "{path}: cannot create macOS path comparison string"
            ));
        }
        let string = CFStringCreateMutableCopy(std::ptr::null(), 0, original);
        CFRelease(original);
        if string.is_null() {
            return Err(format!("{path}: cannot copy macOS path comparison string"));
        }
        CFStringFold(string, 1, std::ptr::null());
        let size = CFStringGetMaximumSizeOfFileSystemRepresentation(string);
        if size <= 0 {
            CFRelease(string);
            return Err(format!("{path}: cannot size macOS path comparison string"));
        }
        let mut bytes = vec![0u8; size as usize];
        let converted =
            CFStringGetFileSystemRepresentation(string, bytes.as_mut_ptr().cast(), size);
        CFRelease(string);
        if converted == 0 {
            return Err(format!("{path}: cannot encode macOS filesystem path"));
        }
        let end = bytes
            .iter()
            .position(|b| *b == 0)
            .ok_or_else(|| format!("{path}: missing filesystem string terminator"))?;
        bytes.truncate(end);
        Ok(bytes)
    }
}

#[cfg(not(verus_keep_ghost))]
fn parents(root: &Path, relative: &str, directories: &mut BTreeSet<PathBuf>) -> Result<(), String> {
    let mut path = root.to_path_buf();
    let parts: Vec<_> = relative.split('/').collect();
    for part in &parts[..parts.len() - 1] {
        path.push(part);
        match metadata(&path)? {
            Some(meta) if !meta.file_type().is_dir() => {
                return Err(error(
                    &path,
                    "expected a directory, not a symlink or other type",
                ));
            }
            None => {
                directories.insert(path.clone());
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(not(verus_keep_ghost))]
fn required(root: &Path, path: &str) -> Result<Vec<u8>, String> {
    let full = root.join(path);
    let snapshot = regular(&full)?.ok_or_else(|| error(&full, "registered source is missing"))?;
    utf8(&full, &snapshot.bytes)?;
    Ok(snapshot.bytes)
}

#[cfg(not(verus_keep_ghost))]
fn raw_catalog(locale: &str) -> Result<Catalog, String> {
    let mut cat: Catalog = catalog::parse(catalog::DEFAULT_CATALOG.as_bytes())?;
    if !cat.locales.iter().any(|l| l.id == locale) {
        return Err(format!("unregistered locale {locale}"));
    }
    cat.atoms = vec![Atom {
        id: "legacy.method".into(),
        requires: vec![],
        variants: vec![Variant {
            locale: locale.into(),
            source: Content::File {
                path: METHOD.into(),
            },
        }],
    }];
    cat.layouts.clear();
    Ok(cat)
}
#[cfg(not(verus_keep_ghost))]
fn raw_manifest(locale: &str) -> Result<Manifest, String> {
    let mut manifest: Manifest = catalog::parse(catalog::DEFAULT_MANIFEST.as_bytes())?;
    let output = &mut manifest.outputs[0];
    output.locale = locale.into();
    output.roots = vec!["legacy.method".into()];
    output.title = None;
    output.layout = None;
    Ok(manifest)
}

#[cfg(not(verus_keep_ghost))]
fn prepare(
    host: &Path,
    mut selected: Option<InitSources>,
) -> Result<(Vec<Change>, Vec<PathBuf>, transaction::Bindings), String> {
    if !cfg!(unix) {
        return Err(
            "root guide publication requires Unix symlinks; this platform is unsupported".into(),
        );
    }
    let root = host.to_path_buf();
    if !root.is_dir() {
        return Err(error(&root, "host root must be an existing directory"));
    }
    let mut directories = BTreeSet::new();
    parents(&root, MANIFEST, &mut directories)?;
    let manifest_path = root.join(MANIFEST);
    let manifest_before = regular(&manifest_path)?;
    let mut changes = Vec::new();
    let mut pending = BTreeMap::<String, Vec<u8>>::new();
    let mut observed_inputs = BTreeMap::new();
    let mut bind = |path: &str, bytes: &[u8]| {
        observed_inputs.insert(path.into(), transaction::sha256(bytes));
    };
    use rule_selection::catalog_admission::{
        PathRole, Registration, accepts_override, alias_conflict, claim_path, descendant_path,
        registration,
    };
    let (manifest, new_manifest) = match registration(manifest_before.as_ref(), selected.as_ref()) {
        Registration::Retained(existing) => {
            bind(MANIFEST, &existing.bytes);
            let manifest: Manifest = catalog::parse(&existing.bytes)
                .map_err(|e| error(&manifest_path, format!("invalid registration: {e}")))?;
            (manifest, false)
        }
        Registration::Missing => {
            return Err(error(
                &manifest_path,
                "missing registration; run explicit init first",
            ));
        }
        Registration::Initialize(sources) => {
            for path in [METHOD, CONTEXT, CATALOG] {
                if regular(&root.join(path))?.is_some() {
                    return Err(error(
                        &root.join(path),
                        "reserved source collision without registration",
                    ));
                }
            }
            let (catalog_bytes, initial) = if let Some(method) = &sources.methodology {
                let locale = sources.locale.as_deref().unwrap_or("und");
                pending.insert(METHOD.into(), method.clone());
                (
                    catalog::bytes(&raw_catalog(locale)?)?,
                    raw_manifest(locale)?,
                )
            } else {
                let mut initial: Manifest = catalog::parse(catalog::DEFAULT_MANIFEST.as_bytes())?;
                if let Some(locale) = &sources.locale {
                    if initial.outputs[0].locale != *locale {
                        initial.outputs[0].title = None;
                    }
                    initial.outputs[0].locale = locale.clone();
                }
                (catalog::DEFAULT_CATALOG.as_bytes().to_vec(), initial)
            };
            pending.insert(CATALOG.into(), catalog_bytes);
            pending.insert(CONTEXT.into(), sources.context.clone().unwrap_or_default());
            (initial, true)
        }
    };
    manifest.validate().map_err(|e| error(&manifest_path, e))?;
    for path in [&manifest.catalog, &manifest.host_context] {
        relative(path, true)?;
        parents(&root, path, &mut directories)?;
    }
    if manifest.catalog == manifest.host_context
        || manifest.catalog == MANIFEST
        || manifest.host_context == MANIFEST
    {
        return Err("source/control overlap in registration".into());
    }
    let catalog_bytes = match pending.get(&manifest.catalog) {
        Some(bytes) => bytes.clone(),
        None => {
            let bytes = required(&root, &manifest.catalog)?;
            bind(&manifest.catalog, &bytes);
            bytes
        }
    };
    let cat: Catalog = catalog::parse(&catalog_bytes).map_err(|e| {
        error(
            &root.join(&manifest.catalog),
            format!("invalid catalog: {e}"),
        )
    })?;
    cat.validate()
        .map_err(|e| error(&root.join(&manifest.catalog), e))?;
    // Distinct control files and every output own one path. File variants may
    // explicitly share an input, but cannot alias a control file or output.
    let mut paths = BTreeSet::new();
    for path in [
        MANIFEST,
        &manifest.catalog,
        &manifest.host_context,
        "AGENTS.md",
    ] {
        claim_path(&mut paths, path.to_owned(), PathRole::Control)
            .map_err(|reason| format!("{path}: {reason}"))?;
    }
    let file_inputs: BTreeSet<_> = cat.file_inputs().into_iter().collect();
    for path in &file_inputs {
        relative(path, true)?;
        claim_path(&mut paths, (*path).to_owned(), PathRole::Source)
            .map_err(|reason| format!("{path}: {reason}"))?;
    }
    for output in &manifest.outputs {
        relative(&output.path, false)?;
        claim_path(&mut paths, output.path.clone(), PathRole::Output)
            .map_err(|reason| format!("{}: {reason}", output.path))?;
    }
    // macOS commonly uses case-insensitive volumes. Refuse inconsistent case
    // or normalization spellings, including prospective directories, even on a case-sensitive
    // macOS volume rather than publishing two ordinary aliases as two owners.
    #[cfg(target_os = "macos")]
    {
        let mut spellings = BTreeMap::new();
        for path in &paths {
            let mut prefix = String::new();
            for component in path.split('/') {
                if !prefix.is_empty() {
                    prefix.push('/');
                }
                prefix.push_str(component);
                let old = spellings.insert(mac_path_key(&prefix)?, prefix.clone());
                if let Some(previous) = alias_conflict(old.as_ref(), &prefix, true) {
                    return Err(format!(
                        "case alias or normalization alias in registered paths: {previous} and {prefix}"
                    ));
                }
            }
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let mut identities = BTreeMap::new();
        for path in &paths {
            if let Some(meta) = metadata(&root.join(path))? {
                if meta.file_type().is_file() {
                    let old = identities.insert((meta.dev(), meta.ino()), path);
                    if let Some(previous) = alias_conflict(old, path, false) {
                        return Err(format!("actual file alias: {previous} and {path}"));
                    }
                }
            }
        }
    }
    for path in &paths {
        // Descendants form one ordered interval beginning at "path/".
        if let Some(other) = paths
            .range(format!("{path}/")..)
            .next()
            .filter(|other| descendant_path(path.as_bytes(), other.as_bytes()))
        {
            return Err(format!("file/ancestor conflict: {path} and {other}"));
        }
        parents(&root, path, &mut directories)?;
    }
    let mut files = BTreeMap::new();
    for path in file_inputs
        .into_iter()
        .chain(std::iter::once(manifest.host_context.as_str()))
    {
        let bytes = match pending.get(path) {
            Some(bytes) => bytes.clone(),
            None => {
                let bytes = required(&root, path)?;
                bind(path, &bytes);
                bytes
            }
        };
        files.insert(path.to_owned(), bytes);
    }
    let external = selected
        .as_mut()
        .map(|s| std::mem::take(&mut s.external))
        .unwrap_or_default();
    if let Some(sources) = selected {
        let conflict = || {
            error(
                &manifest_path,
                "init inputs differ from retained canonical sources or root binding; edit retained sources deliberately and use generate (edit the output plan for locale changes)",
            )
        };
        if !accepts_override(
            sources.context.as_deref(),
            Some(&files[&manifest.host_context]),
        ) || !accepts_override(
            sources.locale.as_deref().map(str::as_bytes),
            Some(manifest.root().locale.as_bytes()),
        ) {
            return Err(conflict());
        }
        if let Some(bytes) = sources.methodology {
            let output = manifest.root();
            let raw = (output.roots.len() == 1)
                .then(|| cat.atoms.iter().find(|a| a.id == output.roots[0]))
                .flatten()
                .filter(|a| a.requires.is_empty())
                .and_then(|a| a.variants.iter().find(|v| v.locale == output.locale))
                .and_then(|v| match &v.source {
                    Content::File { path } => files.get(path),
                    _ => None,
                });
            if !accepts_override(Some(&bytes), raw.map(Vec::as_slice)) {
                return Err(conflict());
            }
        }
    }
    for (path, bytes) in pending {
        changes.push(Change::new(root.join(path), None, bytes));
    }
    let mut rendered = Vec::new();
    for output in &manifest.outputs {
        let catalog::Composition {
            markdown: body,
            locale,
            ..
        } = cat.compose(output, &files)?;
        if output.format == Format::RootGuide {
            rendered.push((output, managed_block(&manifest, &locale.root_frame, &body)));
        } else {
            rendered.push((output, output.projection(&locale.projection_notice, &body)));
        }
    }
    let block = &rendered
        .iter()
        .find(|(o, _)| o.format == Format::RootGuide)
        .unwrap()
        .1;
    let alias = root_guides(&root, block, &mut changes)?;
    for (output, after) in rendered {
        if output.format == Format::RootGuide {
            continue;
        }
        let path = root.join(&output.path);
        let before = regular(&path)?;
        if let Some(existing) = &before {
            output.check_owned(&existing.bytes)?;
        }
        changes.push(Change::new(path, before, after));
    }
    if new_manifest {
        changes.push(Change::new(
            manifest_path,
            manifest_before,
            catalog::bytes(&manifest)?,
        ));
    }
    let unchanged = changes.iter().filter(|c| c.unchanged()).collect();
    let mut bindings = transaction::bind_inputs(&root, observed_inputs, unchanged, alias)?;
    bindings.external = external;
    changes.retain(|c| !c.unchanged());
    let mut directories: Vec<_> = directories.into_iter().collect();
    directories.sort_by_key(|p| p.components().count());
    Ok((changes, directories, bindings))
}

#[cfg(not(verus_keep_ghost))]
fn root_guides(root: &Path, block: &[u8], changes: &mut Vec<Change>) -> Result<bool, String> {
    let agents = root.join("AGENTS.md");
    let claude = root.join("CLAUDE.md");
    let alias = metadata(&agents)?.is_some_and(|m| m.file_type().is_symlink());
    let claude_before = regular(&claude)?;
    let agents_before = if alias {
        if fs::read_link(&agents)
            .map_err(|e| error(&agents, e))?
            .as_os_str()
            != OsStr::new("CLAUDE.md")
            || claude_before.is_none()
        {
            return Err(error(
                &agents,
                "only the relative alias AGENTS.md -> CLAUDE.md with a regular existing target is supported",
            ));
        }
        None
    } else {
        regular(&agents)?
    };
    // Sole AGENTS is a donor. For two regular roots, compare prospective bytes,
    // so stale owned blocks do not cause a conflict but different host text does.
    let donor = claude_before.as_ref().or(agents_before.as_ref());
    let donor_path = if claude_before.is_none() && agents_before.is_some() {
        &agents
    } else {
        &claude
    };
    let after = render(donor_path, donor.map_or(&[], |s| s.bytes.as_slice()), block)?;
    let other = match (&claude_before, &agents_before) {
        (Some(_), Some(before)) => Some(render(&agents, &before.bytes, block)?),
        _ => None,
    };
    let after = rule_selection::consolidate_guides(after, other.as_deref()).map_err(|()| {
        format!(
            "{} and {}: conflicting host-owned content; deliberately consolidate preserved text in CLAUDE.md, then align AGENTS.md or remove the redundant regular AGENTS.md before retrying",
            agents.display(),
            claude.display()
        )
    })?;
    let permissions = donor.map(|s| s.permissions.clone());
    let mut guide = Change::new(claude, claude_before, after);
    guide.permissions = permissions;
    changes.push(guide);
    if !alias {
        changes.push(Change::alias(agents, agents_before));
    }
    Ok(alias)
}

/// Retain exact selected UTF-8 bytes and publish the literal root guide and relative alias.
/// Repeated init refuses differing retained sources. The host must already exist.
#[cfg(not(verus_keep_ghost))]
pub fn init(host: &Path, methodology: &Path, context: &Path) -> Result<Vec<PathBuf>, String> {
    init_with_defaults(host, Some(methodology), Some(context))
}

/// Initialize with independently optional source files. For a new host, omissions
/// select the embedded general core and empty context. For a registered host,
/// omissions retain validated canonical bytes; explicit conflicts still fail.
#[cfg(not(verus_keep_ghost))]
pub fn init_with_defaults(
    host: &Path,
    methodology: Option<&Path>,
    context: Option<&Path>,
) -> Result<Vec<PathBuf>, String> {
    run_init(host, methodology, context, None, Fault::default())
}

#[cfg(not(verus_keep_ghost))]
fn run_init(
    host: &Path,
    methodology: Option<&Path>,
    context: Option<&Path>,
    locale: Option<&OsStr>,
    fault: Fault,
) -> Result<Vec<PathBuf>, String> {
    transaction::run(host, fault, |root| {
        let mut selected = InitSources::read(methodology, context)?;
        selected.locale = locale
            .map(|l| l.to_str().ok_or("locale must be UTF-8").map(str::to_owned))
            .transpose()?;
        prepare(root, Some(selected))
    })
}

/// Validate registered sources and refresh the guide and alias while preserving host-owned bytes.
#[cfg(not(verus_keep_ghost))]
pub fn generate(host: &Path) -> Result<Vec<PathBuf>, String> {
    run_generate(host, Fault::default())
}

#[cfg(not(verus_keep_ghost))]
fn run_generate(host: &Path, fault: Fault) -> Result<Vec<PathBuf>, String> {
    transaction::run(host, fault, |root| prepare(root, None))
}

/// Bounded IO-failure injection available only in the dedicated test build.
#[cfg(feature = "test-support")]
#[cfg(not(verus_keep_ghost))]
pub mod test_support {
    /// Observe one isolated consumer operation on this thread.
    pub fn with_repair_observation<T>(operation: impl FnOnce() -> T) -> (T, bool) {
        super::publication_decision::observation::observe(operation)
    }

    pub fn publish_catalog(catalog: &[u8], manifest: &[u8]) -> Result<String, String> {
        let catalog = super::catalog::parse(catalog)?;
        let manifest = super::catalog::parse(manifest)?;
        super::inherit::publish_catalog(&catalog, &manifest)
    }

    pub fn read_rules(
        root: &std::path::Path,
        fetch: impl FnOnce() -> Result<Vec<u8>, String>,
    ) -> Result<String, String> {
        super::inherit::read(root, fetch)
    }

    pub fn sync_rules(
        root: &std::path::Path,
        fetch: impl FnOnce() -> Result<Vec<u8>, String>,
    ) -> Result<Vec<std::path::PathBuf>, String> {
        super::inherit::sync(root, fetch)
    }

    use super::*;

    pub fn generate_with_failure(
        host: &Path,
        after: usize,
        rollback: Option<usize>,
    ) -> Result<Vec<PathBuf>, String> {
        run_generate(
            host,
            Fault {
                after: Some(after),
                rollback,
                ..Fault::default()
            },
        )
    }

    pub fn init_with_failure(
        host: &Path,
        method: &Path,
        context: &Path,
        after: usize,
        rollback: Option<usize>,
    ) -> Result<Vec<PathBuf>, String> {
        run_init(
            host,
            Some(method),
            Some(context),
            None,
            Fault {
                after: Some(after),
                rollback,
                ..Fault::default()
            },
        )
    }

    pub fn init_with_checkpoint(
        host: &Path,
        method: &Path,
        context: &Path,
        hook: fn(&str),
    ) -> Result<Vec<PathBuf>, String> {
        run_init(
            host,
            Some(method),
            Some(context),
            None,
            Fault {
                checkpoint: Some(hook),
                ..Fault::default()
            },
        )
    }
}

/// CLI transport result; exit 0 complete, 2 usage, 1 input/generation/IO failure.
#[derive(Debug)]
#[cfg(not(verus_keep_ghost))]
pub struct CliOutput {
    pub exit_code: u8,
    pub stdout: String,
    pub stderr: String,
}

/// Accept OS paths without shell interpolation or UTF-8 conversion of arguments.
#[cfg(not(verus_keep_ghost))]
pub fn dispatch(args: &[OsString]) -> CliOutput {
    let reply = |exit_code, stdout: String, stderr: String| CliOutput {
        exit_code,
        stdout,
        stderr,
    };
    let single = match args {
        [op] => op.to_str(),
        _ => None,
    };
    if let Some(op @ ("event-regex" | "event-read" | "event-format")) = single {
        return match github_event::command(op, std::io::stdin().lock()) {
            Ok(output) => reply(0, output, String::new()),
            Err(error) => reply(1, String::new(), format!("E_GITHUB_EVENT: {error}\n")),
        };
    }
    let rule_result = match args {
        [op, flag, host] if op == "sync-rules" && flag == "--host-root" => {
            Some(inherit::sync(Path::new(host), inherit::fetch).map(|_| String::new()))
        }
        [op] if op == "publish-rules" => Some(inherit::publish()),
        [op, flag, host] if op == "rules" && flag == "--host-root" => {
            Some(inherit::read(Path::new(host), inherit::fetch))
        }
        _ => None,
    };
    if let Some(result) = rule_result {
        return match result {
            Ok(text) => reply(0, text, String::new()),
            Err(error) => reply(1, String::new(), format!("E_RULES: {error}\n")),
        };
    }
    if args.is_empty()
        || (args.len() == 1 && ["help", "--help", "-h"].iter().any(|a| args[0] == *a))
    {
        return reply(0, "Usage:\n  chrono-instructions event-regex | event-read | event-format\n  chrono-instructions sync-rules --host-root H\n  chrono-instructions rules --host-root H\n  chrono-instructions publish-rules\n  chrono-instructions init --host-root H [--methodology M] [--host-context C] [--locale L]\n  chrono-instructions generate --host-root H\nH must be an existing directory. New init adopts embedded atomic rules (zh-CN default) and empty context. --locale binds the fresh root; --methodology retains one opaque file atom (und unless explicitly bound). Registered init retains existing canonical sources for omitted options; explicit differing inputs fail. Optional M/C must be readable UTF-8 regular files. Edit the registered catalog and output plan, then generate root guides, Markdown and skills. AGENTS.md -> CLAUDE.md remains a literal relative alias. Unix only. No host inference or runtime checkout required. Exit 0 complete, 2 usage, 1 generation/IO failure. No judges run.\n".into(), String::new());
    }
    if args.len() == 1 && args[0] == "--version" {
        return reply(
            0,
            concat!("chrono-instructions ", env!("CARGO_PKG_VERSION"), "\n").into(),
            String::new(),
        );
    }
    let usage = || {
        reply(
            2,
            String::new(),
            "E_USAGE: use --help; --host-root is required, each option may occur only once and needs a value.\n".into(),
        )
    };
    let is_init = args[0] == "init";
    if !is_init && args[0] != "generate" {
        return usage();
    }
    let mut host = None;
    let mut method = None;
    let mut context = None;
    let mut locale = None;
    let mut parts = args[1..].chunks_exact(2);
    for pair in &mut parts {
        let slot = if pair[0] == "--host-root" {
            &mut host
        } else if is_init && pair[0] == "--methodology" {
            &mut method
        } else if is_init && pair[0] == "--host-context" {
            &mut context
        } else if is_init && pair[0] == "--locale" {
            &mut locale
        } else {
            return usage();
        };
        if slot.is_some() || pair[1].is_empty() {
            return usage();
        }
        *slot = Some(PathBuf::from(&pair[1]));
    }
    if !parts.remainder().is_empty() {
        return usage();
    }
    let Some(host) = host else {
        return usage();
    };
    let result = if is_init {
        run_init(
            &host,
            method.as_deref(),
            context.as_deref(),
            locale.as_deref().map(Path::as_os_str),
            Fault::default(),
        )
    } else {
        generate(&host)
    };
    match result {
        Ok(paths) => reply(
            0,
            format!(
                "complete: {} file(s) changed; no judges executed\n",
                paths.len()
            ),
            String::new(),
        ),
        Err(e) => reply(1, String::new(), format!("E_GENERATION: {e}\n")),
    }
}
