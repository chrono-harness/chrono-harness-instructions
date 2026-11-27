//! An opted-in host publishes events using its installed chrono-diagnostics.
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
/// A completed logging session returns the business code or the owner's failure code.
pub fn session_exit(business: u8, status: Option<i32>) -> (exit: Option<u8>)
    ensures exit == if status == Some(business as i32) { Some(business) }
                    else if status == Some(2i32) { Some(2u8) } else { None },
{
    match status {
        Some(code) if code == business as i32 => Some(business),
        Some(2) => Some(2),
        _ => None,
    }
}

}

#[cfg(not(verus_keep_ghost))]
pub(crate) use native::*;
#[cfg(not(verus_keep_ghost))]
mod native {
    use chrono_instructions::CliOutput;
    use std::ffi::{OsStr, OsString};
    use std::fs;
    use std::io::{self, BufRead, BufReader, Read, Write};
    use std::path::Path;
    use std::process::{Command, Stdio};

    use serde_json::json;

    pub(crate) enum StartError {
        Reported,
        Transport(io::Error),
    }

    impl From<io::Error> for StartError {
        fn from(error: io::Error) -> Self {
            Self::Transport(error)
        }
    }

    type Completed = (CliOutput, io::Result<u8>);

    pub(crate) fn host(args: &[OsString]) -> &Path {
        args.windows(2)
            .find(|pair| pair[0] == "--host-root")
            .map_or(Path::new("."), |pair| Path::new(&pair[1]))
    }

    pub(crate) fn ready(reader: impl Read) -> io::Result<()> {
        let mut line = Vec::new();
        BufReader::new(reader)
            .take(256)
            .read_until(b'\n', &mut line)?;
        if line.ends_with(b"\n")
            && serde_json::from_slice::<serde_json::Value>(&line)? == json!({"ready": true})
        {
            Ok(())
        } else {
            Err(io::Error::other("chrono-diagnostics did not report ready"))
        }
    }

    pub(crate) fn complete(mut input: impl Write, message: &str, exit: u8) -> io::Result<()> {
        if !message.is_empty() {
            serde_json::to_writer(&mut input, &json!({"diagnostic": message.trim_end()}))?;
            input.write_all(b"\n")?;
        }
        serde_json::to_writer(&mut input, &json!({"exit_code": exit}))?;
        input.write_all(b"\n")
    }

    pub(crate) fn command(args: &[OsString], pid: u32) -> io::Result<Option<Command>> {
        let root = host(args);
        match fs::symlink_metadata(root.join(".chrono-harness/logging.json")) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
            Ok(_) => {}
        }
        let operation = args.first().map_or(OsStr::new("help"), OsString::as_os_str);
        let mut command = Command::new("chrono-diagnostics");
        command
            .arg("cli-events-v1")
            .arg(root.canonicalize()?)
            .arg("chrono-instructions")
            .arg(operation.to_string_lossy().as_ref())
            .arg(pid.to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        Ok(Some(command))
    }

    pub(crate) fn communicate(
        input: impl Write,
        output: impl Read,
        wait: impl FnOnce() -> io::Result<Option<i32>>,
        work: impl FnOnce() -> CliOutput,
    ) -> Result<Completed, StartError> {
        if let Err(error) = ready(output) {
            drop(input);
            return match wait()? {
                // The logging owner has already written its startup diagnostic to stderr.
                Some(2) => Err(StartError::Reported),
                status => {
                    Err(io::Error::other(format!("chrono-diagnostics {status:?}: {error}")).into())
                }
            };
        }
        let result = work();
        // Close stdin before joining even if the diagnostic or terminal message cannot be sent.
        let sent = complete(input, &result.stderr, result.exit_code);
        let status = wait();
        let exit = sent.and_then(|()| {
            let status = status?;
            super::session_exit(result.exit_code, status).ok_or_else(|| {
                io::Error::other(format!("unexpected chrono-diagnostics status {status:?}"))
            })
        });
        Ok((result, exit))
    }

    pub(crate) fn run(args: &[OsString]) -> Result<Completed, StartError> {
        let Some(mut command) = command(args, std::process::id())? else {
            let result = chrono_instructions::dispatch(args);
            let exit = result.exit_code;
            return Ok((result, Ok(exit)));
        };
        let mut child = command.spawn()?;
        let input = child.stdin.take().expect("requested stdin pipe");
        let output = child.stdout.take().expect("requested stdout pipe");
        communicate(
            input,
            output,
            || child.wait().map(|s| s.code()),
            || chrono_instructions::dispatch(args),
        )
    }

    /// Emergency output when the external logging process itself cannot be used.
    pub(crate) fn diagnostic(code: &str, message: &str, exit: Option<u8>) {
        let row = json!({"schema":"chrono-log/v1","producer":"chrono-instructions","event":"runtime.logging_failed",
        "error":{"error":{"code":code,"message":message,"context":{"operation_exit_code":exit}}}});
        let _ = writeln!(io::stderr().lock(), "{row}");
    }
}
