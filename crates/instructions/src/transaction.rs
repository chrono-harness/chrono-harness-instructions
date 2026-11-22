use std::fs::{self, Permissions};
use std::path::{Path, PathBuf};

pub(super) struct Snapshot {
    pub bytes: Vec<u8>,
    pub permissions: Permissions,
    pub identity: String,
}

pub(super) struct Change {
    pub path: PathBuf,
    pub before: Option<Snapshot>,
    pub after: After,
    pub permissions: Option<Permissions>,
}

pub(super) enum After {
    Regular(Vec<u8>),
    RelativeAlias,
}

impl Change {
    pub fn new(path: PathBuf, before: Option<Snapshot>, after: Vec<u8>) -> Self {
        let permissions = before.as_ref().map(|s| s.permissions.clone());
        Self {
            path,
            before,
            after: After::Regular(after),
            permissions,
        }
    }

    pub fn alias(path: PathBuf, before: Option<Snapshot>) -> Self {
        Self {
            path,
            before,
            after: After::RelativeAlias,
            permissions: None,
        }
    }

    pub fn unchanged(&self) -> bool {
        matches!((&self.before, &self.after), (Some(before), After::Regular(after)) if before.bytes == *after)
    }
}

use crate::publication_decision::{Repair, repair};
use rustix::fs::{CWD, RenameFlags, renameat_with};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Write;
use std::os::unix::{
    ffi::{OsStrExt, OsStringExt},
    fs::{MetadataExt, OpenOptionsExt, PermissionsExt, symlink},
};

pub(super) fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(super) fn identity(path: &Path) -> Result<String, String> {
    let meta = fs::symlink_metadata(path).map_err(|err| super::error(path, err))?;
    Ok(format!(
        "{}:{}:{}",
        meta.dev(),
        meta.ino(),
        meta.mode() & libc::S_IFMT as u32
    ))
}

fn publish_entry(source: &Path, target: &Path) -> Result<(), String> {
    renameat_with(CWD, source, CWD, target, RenameFlags::NOREPLACE)
        .map_err(|err| super::error(target, err))
}

#[derive(Default)]
pub(super) struct Fault {
    pub after: Option<usize>,
    pub rollback: Option<usize>,
    pub checkpoint: Option<fn(&str)>,
    pub fail: Option<String>,
}

impl Fault {
    fn at(&self, point: &str) -> Result<(), String> {
        if let Some(hook) = self.checkpoint {
            hook(point);
        }
        if self.fail.as_deref() == Some(point) {
            return Err(format!("injected {point} state/cleanup failure"));
        }
        Ok(())
    }
}

struct Plan {
    host: String,
    directories: Vec<Entry>,
    files: Vec<Entry>,
    parents: BTreeMap<String, Object>,
    bindings: Bindings,
}

#[derive(Default)]
pub(super) struct Bindings {
    inputs: BTreeMap<String, Object>,
    unchanged: BTreeMap<String, Option<Object>>,
    pub external: Vec<ExternalInput>,
}

pub(super) struct ExternalInput {
    requested: Vec<u8>,
    resolved: Vec<u8>,
    identity: String,
    sha256: String,
    mode: u32,
}

pub(super) fn external_input(path: &Path) -> Result<(Vec<u8>, ExternalInput), String> {
    let requested = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(path)
    };
    let resolved = fs::canonicalize(&requested).map_err(|e| super::error(path, e))?;
    let meta = fs::metadata(&resolved).map_err(|e| super::error(path, e))?;
    if !meta.is_file() {
        return Err(super::error(path, "expected a readable regular input file"));
    }
    let id = identity(&resolved)?;
    let bytes = fs::read(&resolved).map_err(|e| super::error(path, e))?;
    if identity(&resolved)? != id
        || stamp(&fs::metadata(&resolved).map_err(|e| e.to_string())?) != stamp(&meta)
    {
        return Err(super::error(path, "external input changed while reading"));
    }
    let binding = ExternalInput {
        requested: requested.as_os_str().as_bytes().to_vec(),
        resolved: resolved.as_os_str().as_bytes().to_vec(),
        identity: id,
        sha256: sha256(&bytes),
        mode: mode(&meta),
    };
    Ok((bytes, binding))
}

pub(super) fn bind_inputs(
    root: &Path,
    inputs: BTreeMap<String, String>,
    unchanged: Vec<&Change>,
    alias: bool,
) -> Result<Bindings, String> {
    let mut bound = Bindings::default();
    for (name, digest) in inputs {
        let path = target(root, &name)?;
        let object =
            observe(&path)?.ok_or_else(|| super::error(&path, "registered input disappeared"))?;
        if object.bytes.as_ref() != Some(&digest) {
            return Err(super::error(
                &path,
                "registered input changed during preparation",
            ));
        }
        bound.inputs.insert(name, object);
    }
    for change in unchanged {
        let expected = change.before.as_ref().map(|b| Object {
            identity: b.identity.clone(),
            bytes: Some(sha256(&b.bytes)),
            mode: b.permissions.mode() & 0o777,
        });
        if observe(&change.path)? != expected {
            return Err(super::error(
                &change.path,
                "unchanged destination changed during preparation",
            ));
        }
        bound
            .unchanged
            .insert(relative(root, &change.path)?, expected);
    }
    if alias {
        let path = root.join("AGENTS.md");
        let expected = observe(&path)?.ok_or("literal alias disappeared during preparation")?;
        if !fs::symlink_metadata(&path)
            .map_err(|e| e.to_string())?
            .is_symlink()
            || expected.bytes.as_ref() != Some(&sha256(b"CLAUDE.md"))
        {
            return Err(super::error(
                &path,
                "literal alias changed during preparation",
            ));
        }
        bound.unchanged.insert("AGENTS.md".into(), Some(expected));
    }
    Ok(bound)
}

struct Entry {
    target: String,
    before: Option<Object>,
    next: Object,
    backup: Option<Object>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Object {
    identity: String,
    bytes: Option<String>,
    mode: u32,
}

fn mode(meta: &fs::Metadata) -> u32 {
    meta.mode() & 0o777
}

fn stamp(meta: &fs::Metadata) -> (u64, i64, i64, i64, i64, u32, u64) {
    (
        meta.len(),
        meta.mtime(),
        meta.mtime_nsec(),
        meta.ctime(),
        meta.ctime_nsec(),
        meta.mode(),
        meta.nlink(),
    )
}

fn observe(path: &Path) -> Result<Option<Object>, String> {
    let Some(meta) = super::metadata(path)? else {
        return Ok(None);
    };
    let id = identity(path)?;
    let bytes = if meta.is_file() && meta.nlink() == 1 {
        Some(sha256(&fs::read(path).map_err(|e| super::error(path, e))?))
    } else if meta.is_symlink() {
        Some(sha256(
            fs::read_link(path)
                .map_err(|e| super::error(path, e))?
                .as_os_str()
                .as_bytes(),
        ))
    } else if meta.is_dir() {
        None
    } else {
        return Err(super::error(
            path,
            "unexpected publication object type or hard link",
        ));
    };
    if identity(path)? != id
        || (!meta.is_dir()
            && stamp(&fs::symlink_metadata(path).map_err(|e| e.to_string())?) != stamp(&meta))
    {
        return Err(super::error(
            path,
            "publication object identity/content changed during read",
        ));
    }
    Ok(Some(Object {
        identity: id,
        bytes,
        mode: mode(&meta),
    }))
}

fn sync(path: &Path) -> Result<(), String> {
    fs::File::open(path)
        .and_then(|f| f.sync_all())
        .map_err(|e| super::error(path, e))
}

fn write(path: &Path, bytes: &[u8], mode: u32) -> Result<(), String> {
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|e| super::error(path, e))?;
    f.write_all(bytes)
        .and_then(|()| f.set_permissions(Permissions::from_mode(mode)))
        .and_then(|()| f.sync_all())
        .map_err(|e| super::error(path, e))
}

fn relative(root: &Path, path: &Path) -> Result<String, String> {
    let name = path
        .strip_prefix(root)
        .ok()
        .and_then(Path::to_str)
        .ok_or_else(|| super::error(path, "invalid host-relative publication path"))?;
    super::relative(name, false)?;
    Ok(name.into())
}

fn target(root: &Path, name: &str) -> Result<PathBuf, String> {
    super::relative(name, false)?;
    let path = root.join(name);
    super::parents(root, name, &mut Default::default())?;
    Ok(path)
}

fn build_plan(
    root: &Path,
    directory: &Path,
    changes: Vec<Change>,
    directories: Vec<PathBuf>,
    bindings: Bindings,
    byte_limit: u64,
    fault: &Fault,
) -> Result<Plan, String> {
    let mut parents = BTreeMap::new();
    let mut dir_entries = Vec::new();
    let mut files = Vec::new();
    let mut total = 0u64;
    for change in &changes {
        total = total.saturating_add(change.before.as_ref().map_or(0, |b| b.bytes.len() as u64));
        if let After::Regular(bytes) = &change.after {
            total = total.saturating_add(bytes.len() as u64);
        }
    }
    if total > byte_limit {
        return Err("instruction staging exceeds declared output byte limit".into());
    }
    for destination in directories.iter().chain(changes.iter().map(|c| &c.path)) {
        for parent in destination.ancestors().skip(1).take_while(|p| *p != root) {
            if !directories.iter().any(|path| path == parent) {
                parents.insert(
                    relative(root, parent)?,
                    observe(parent)?.ok_or("publication parent disappeared")?,
                );
            }
        }
    }
    for (i, path) in directories.into_iter().enumerate() {
        let name = relative(root, &path)?;
        if super::metadata(&target(root, &name)?)?.is_some() {
            return Err(super::error(
                &path,
                "directory destination changed during preparation",
            ));
        }
        let staged = directory.join(format!("dir-{i}"));
        fs::create_dir(&staged).map_err(|e| super::error(&staged, e))?;
        dir_entries.push(Entry {
            target: name,
            before: None,
            next: observe(&staged)?.unwrap(),
            backup: None,
        });
        fault.at(&format!("staged-dir-{i}"))?;
    }
    for (i, change) in changes.into_iter().enumerate() {
        let name = relative(root, &change.path)?;
        let before = change.before.as_ref().map(|b| Object {
            identity: b.identity.clone(),
            bytes: Some(sha256(&b.bytes)),
            mode: b.permissions.mode() & 0o777,
        });
        if observe(&target(root, &name)?)? != before {
            return Err(super::error(
                &change.path,
                "destination changed during preparation",
            ));
        }
        let next = directory.join(format!("next-{i}"));
        match change.after {
            After::Regular(bytes) => write(
                &next,
                &bytes,
                change.permissions.map_or(0o600, |p| p.mode() & 0o777),
            )?,
            After::RelativeAlias => {
                symlink("CLAUDE.md", &next).map_err(|e| super::error(&next, e))?
            }
        }
        let backup = if let Some(before) = change.before {
            let path = directory.join(format!("original-{i}"));
            write(&path, &before.bytes, before.permissions.mode() & 0o777)?;
            observe(&path)?
        } else {
            None
        };
        files.push(Entry {
            target: name,
            before,
            next: observe(&next)?.unwrap(),
            backup,
        });
        fault.at(&format!("staged-file-{i}"))?;
    }
    Ok(Plan {
        host: identity(root)?,
        directories: dir_entries,
        files,
        parents,
        bindings,
    })
}

fn check_parents(root: &Path, plan: &Plan, path: &Path) -> Result<(), String> {
    if identity(root)? != plan.host {
        return Err("publication host identity changed; preserve state".into());
    }
    for parent in path.ancestors().skip(1).take_while(|p| *p != root) {
        let name = relative(root, parent)?;
        let observed = super::metadata(parent)?;
        let current = observed.as_ref().map(|_| identity(parent)).transpose()?;
        if let Some(expected) = plan.parents.get(&name) {
            if !observed.is_some_and(|m| m.is_dir()) || observe(parent)?.as_ref() != Some(expected)
            {
                return Err(super::error(
                    parent,
                    "original publication parent changed; preserve pending state",
                ));
            }
        } else if let Some(created) = plan.directories.iter().find(|e| e.target == name) {
            if current
                .as_ref()
                .is_some_and(|id| *id != created.next.identity)
            {
                return Err(super::error(
                    parent,
                    "created publication parent changed; preserve pending state",
                ));
            }
        } else {
            return Err(super::error(parent, "unbound publication parent"));
        }
    }
    Ok(())
}

fn check_bindings(root: &Path, bindings: &Bindings) -> Result<(), String> {
    for (name, expected) in &bindings.inputs {
        let path = target(root, name)?;
        if observe(&path)?.as_ref() != Some(expected) {
            return Err(super::error(
                &path,
                "registered input changed after preparation",
            ));
        }
    }
    for (name, expected) in &bindings.unchanged {
        let path = target(root, name)?;
        if observe(&path)? != *expected {
            return Err(super::error(
                &path,
                "unchanged destination or literal alias changed",
            ));
        }
    }
    for external in &bindings.external {
        let requested = PathBuf::from(std::ffi::OsString::from_vec(external.requested.clone()));
        let (_, observed) = external_input(&requested)?;
        if observed.resolved != external.resolved
            || observed.identity != external.identity
            || observed.sha256 != external.sha256
            || observed.mode != external.mode
        {
            return Err(super::error(
                &requested,
                "external input changed after preparation",
            ));
        }
    }
    Ok(())
}

fn install(root: &Path, plan: &Plan, directory: &Path, fault: &Fault) -> Result<(), String> {
    check_bindings(root, &plan.bindings)?;
    for (entries, prefix) in [(&plan.directories, "dir"), (&plan.files, "next")] {
        for (i, entry) in entries.iter().enumerate() {
            let path = target(root, &entry.target)?;
            check_parents(root, plan, &path)?;
            let staged = directory.join(format!("{prefix}-{i}"));
            if observe(&path)? != entry.before || observe(&staged)?.as_ref() != Some(&entry.next) {
                return Err(super::error(
                    &path,
                    "staging/destination drift; preserve pending state",
                ));
            }
            if prefix == "next" && fault.after == Some(i) {
                return Err(super::error(&path, "injected publication failure"));
            }
            if entry.before.is_some() {
                fs::rename(&staged, &path).map_err(|e| super::error(&path, e))?;
            } else {
                publish_entry(&staged, &path)?;
            }
            sync(path.parent().unwrap())?;
            sync(directory)?;
            if prefix == "next" {
                fault.at(&format!("published-{i}"))?;
            } else {
                fault.at(&format!("published-dir-{i}"))?;
            }
        }
    }
    Ok(())
}

fn settle_effects(
    root: &Path,
    directory: &Path,
    plan: &Plan,
    committed: bool,
    fault: &Fault,
) -> Result<Vec<PathBuf>, String> {
    let mut unrestored = Vec::new();
    let mut recovery = Vec::new();
    let mut published = Vec::new();
    for (entries, is_file) in [(&plan.files, true), (&plan.directories, false)] {
        for (i, entry) in entries.iter().enumerate().rev() {
            let path = root.join(&entry.target);
            let backup = directory.join(format!("original-{i}"));
            let action: Result<(), String> = (|| {
                let path = target(root, &entry.target)?;
                check_parents(root, plan, &path)?;
                let current = observe(&path)?;
                let before =
                    current == entry.before || (entry.backup.is_some() && current == entry.backup);
                let installed = current.as_ref() == Some(&entry.next);
                if installed && is_file {
                    published.push(path.clone());
                }
                let ready = entry.backup.is_none() || observe(&backup)? == entry.backup;
                match repair(before, installed, ready, committed) {
                    Repair::Leave => Ok(()),
                    Repair::Preserve => {
                        Err("destination or original identity/content drift; preserve it".into())
                    }
                    Repair::Restore => {
                        if is_file && fault.rollback == Some(i) {
                            return Err("injected rollback failure".into());
                        }
                        if !is_file
                            && unrestored
                                .iter()
                                .any(|unrestored: &PathBuf| unrestored.starts_with(&path))
                        {
                            return Ok(());
                        }
                        if entry.backup.is_some() {
                            fs::rename(&backup, &path).map_err(|e| super::error(&path, e))?;
                        } else if is_file {
                            fs::remove_file(&path).map_err(|e| super::error(&path, e))?;
                        } else {
                            fs::remove_dir(&path).map_err(|e| super::error(&path, e))?;
                        }
                        sync(path.parent().unwrap())?;
                        sync(directory)?;
                        fault.at(&format!(
                            "restored-{}-{i}",
                            if is_file { "file" } else { "dir" }
                        ))?;
                        Ok(())
                    }
                }
            })();
            if let Err(error) = action {
                if entry.backup.is_some() && backup.is_file() {
                    recovery.push(format!("original retained at {}", backup.display()));
                }
                recovery.push(format!("{}: {error}", path.display()));
                unrestored.push(path);
            }
        }
    }
    if unrestored.is_empty() {
        Ok(published)
    } else {
        Err(format!(
            "published={published:?}; unrestored={unrestored:?}; recovery={recovery:?}"
        ))
    }
}

// The advisory lock belongs to the host directory. Explicit unlock also
// releases inherited copies of the same open file description.
struct HostLock(fs::File);

impl Drop for HostLock {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}

pub(super) fn run(
    host: &Path,
    fault: Fault,
    prepare: impl FnOnce(&Path) -> Result<(Vec<Change>, Vec<PathBuf>, Bindings), String>,
) -> Result<Vec<PathBuf>, String> {
    let root = host.canonicalize().map_err(|e| super::error(host, e))?;
    let file = fs::File::open(&root).map_err(|e| super::error(&root, e))?;
    file.try_lock()
        .map_err(|e| format!("instructions operation busy or unavailable on this host: {e}"))?;
    let _lock = HostLock(file);
    let (changes, directories, bindings) = prepare(&root)?;
    if changes.is_empty() && directories.is_empty() {
        check_bindings(&root, &bindings)?;
        return Ok(vec![]);
    }
    let stage = tempfile::Builder::new()
        .prefix(".chrono-instructions-")
        .tempdir_in(&root)
        .map_err(|e| super::error(&root, e))?;
    let plan = build_plan(
        &root,
        stage.path(),
        changes,
        directories,
        bindings,
        64 * 1024 * 1024,
        &fault,
    )?;
    fault.at("prepared")?;
    let publication = install(&root, &plan, stage.path(), &fault);
    if let Err(err) = publication {
        return match settle_effects(&root, stage.path(), &plan, false, &fault) {
            Ok(published) => Err(format!("{err}; published={published:?}; unrestored=[]")),
            Err(repair) => {
                let retained = stage.keep();
                Err(format!(
                    "{err}; {repair}; original staging retained at {}",
                    retained.display()
                ))
            }
        };
    }
    if let Err(reason) = settle_effects(&root, stage.path(), &plan, true, &fault) {
        let retained = stage.keep();
        return Err(format!(
            "{reason}; original staging retained at {}",
            retained.display()
        ));
    }
    stage
        .close()
        .map_err(|e| format!("instructions published; temporary cleanup failed: {e}"))?;
    Ok(plan
        .files
        .iter()
        .map(|entry| root.join(&entry.target))
        .collect())
}
