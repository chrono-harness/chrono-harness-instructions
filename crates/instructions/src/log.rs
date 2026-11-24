//! Optional, bounded instruction-command logs. Each tool owns its own files.
use chrono_instructions::publication_decision::{LogWrite, event_fits, log_write};
use file_rotate::{ContentLimit, FileRotate, compression::Compression, suffix::AppendCount};
use serde::Deserialize;
use serde_json::json;
use std::{
    ffi::OsString,
    fs,
    io::{self, Write},
    path::{Component, Path, PathBuf},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Policy {
    schema: String,
    level: String,
    directory: String,
    max_bytes: usize,
    max_files: usize,
    stderr: bool,
}

pub(crate) struct Log {
    policy: Policy,
    host: PathBuf,
    operation: OsString,
    directory: PathBuf,
}

fn directory_under(root: &Path, path: &Path) -> io::Result<PathBuf> {
    let mut directory = root.to_owned();
    for part in path.components() {
        let Component::Normal(name) = part else {
            return Err(io::Error::other("invalid log directory"));
        };
        directory.push(name);
        match fs::create_dir(&directory) {
            Ok(()) => (),
            Err(e)
                if e.kind() == io::ErrorKind::AlreadyExists
                    && fs::symlink_metadata(&directory)?.is_dir() =>
            {
                ()
            }
            Err(e) => return Err(e),
        }
    }
    Ok(directory)
}

impl Log {
    pub(crate) fn for_args(args: &[OsString]) -> Result<Option<Self>, String> {
        let root = args
            .windows(2)
            .find(|p| p[0] == "--host-root")
            .map_or(Path::new("."), |p| Path::new(&p[1]));
        let config = root.join(".chrono-harness/logging.json");
        let bytes = match fs::read(&config) {
            Ok(b) => b,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.to_string()),
        };
        let policy: Policy = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        let dir = Path::new(&policy.directory);
        if policy.schema != "chrono-runtime-logging/v1"
            || !(1..=64).contains(&policy.max_files)
            || !(1..=64 * 1024 * 1024).contains(&policy.max_bytes)
            || !matches!(
                policy.level.as_str(),
                "trace" | "debug" | "info" | "warn" | "error"
            )
            || !dir.starts_with(".chrono-harness")
            || !dir.components().all(|c| matches!(c, Component::Normal(_)))
        {
            return Err("invalid log policy or bounds".into());
        }
        let host = root.canonicalize().map_err(|e| e.to_string())?;
        let directory =
            directory_under(&host, &dir.join("chrono-instructions")).map_err(|e| e.to_string())?;
        Ok(Some(Self {
            policy,
            host,
            operation: args.first().cloned().unwrap_or_default(),
            directory,
        }))
    }

    pub(crate) fn event(
        &self,
        event: &str,
        message: Option<&str>,
        exit: Option<u8>,
    ) -> io::Result<()> {
        let failed = message.is_some() || exit.is_some_and(|code| code != 0);
        if !failed && matches!(self.policy.level.as_str(), "warn" | "error") {
            return Ok(());
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(io::Error::other)?;
        let row = json!({"schema":"chrono-log/v1","producer":"chrono-instructions","event":event,
            "level":if failed {"error"} else {"info"}, "unix_millis":now.as_millis(),
            "context":{"pid":std::process::id(),"host_root":self.host.to_string_lossy(),"operation":self.operation.to_string_lossy()},
            "message":message,"fields":{"exit_code":exit}});
        let mut bytes = serde_json::to_vec(&row)?;
        bytes.push(b'\n');
        if !event_fits(bytes.len() as u64, self.policy.max_bytes as u64) {
            return Err(io::Error::other("log event exceeds file size bound"));
        }
        let directory = fs::File::open(&self.directory)?;
        directory.lock()?;
        let path = self.directory.join("runtime.jsonl");
        let length = match fs::symlink_metadata(&path) {
            Ok(meta) if meta.is_file() => meta.len(),
            Ok(_) => return Err(io::Error::other("expected regular log file")),
            Err(e) if e.kind() == io::ErrorKind::NotFound => 0,
            Err(e) => return Err(e),
        };
        match log_write(
            length,
            bytes.len() as u64,
            self.policy.max_bytes as u64,
            self.policy.max_files,
        ) {
            LogWrite::TooLarge => {
                return Err(io::Error::other("log event exceeds file size bound"));
            }
            LogWrite::Append => {}
            LogWrite::Rotate { archived_files } => {
                FileRotate::new(
                    &path,
                    AppendCount::new(archived_files),
                    ContentLimit::None,
                    Compression::None,
                    None,
                )
                .rotate()?;
            }
        }
        use std::os::unix::fs::OpenOptionsExt;
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)?
            .write_all(&bytes)?;
        if self.policy.stderr {
            io::stderr().lock().write_all(&bytes)?;
        }
        Ok(())
    }
}

pub(crate) fn diagnostic(code: &str, message: &str, exit: Option<u8>) {
    let row = json!({"schema":"chrono-log/v1","producer":"chrono-instructions","event":"runtime.logging_failed",
        "error":{"error":{"code":code,"message":message,"context":{"operation_exit_code":exit}}}});
    let _ = writeln!(io::stderr().lock(), "{row}");
}
