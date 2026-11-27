#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
use super::finish_cli;
#[cfg(not(verus_keep_ghost))]
use chrono_instructions::publication_decision::finish_cli;

verus! {
/// A transport failure retains its cause if command output could be delivered.
pub fn stream_result<E>(written: bool, session: Result<u8, E>, business: u8) -> (out: (u8, Option<E>))
    ensures out == if !written { (1u8, None) }
        else { match session { Ok(code) => (code, None), Err(e) => (2u8, Some(e)) } },
{
    match session {
        Ok(code) => finish_cli(written, None, code),
        Err(error) => finish_cli(written, Some(error), business),
    }
}
}

#[cfg(not(verus_keep_ghost))]
use std::io::{self, Write};
#[cfg(not(verus_keep_ghost))]
use std::process::ExitCode;

#[cfg(not(verus_keep_ghost))]
mod log;

#[cfg(not(verus_keep_ghost))]
fn main() -> ExitCode {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    let (result, session) = match log::run(&args) {
        Ok(result) => result,
        Err(log::StartError::Reported) => return ExitCode::from(2),
        Err(log::StartError::Transport(error)) => {
            log::diagnostic("E_LOG_CONFIG", &error.to_string(), None);
            return ExitCode::from(2);
        }
    };
    let streams_written = io::stdout()
        .lock()
        .write_all(result.stdout.as_bytes())
        .is_ok()
        && io::stderr()
            .lock()
            .write_all(result.stderr.as_bytes())
            .is_ok();
    let (exit, diagnostic) = stream_result(streams_written, session, result.exit_code);
    if let Some(error) = diagnostic {
        log::diagnostic("E_LOG_WRITE", &error.to_string(), Some(result.exit_code));
    }
    ExitCode::from(exit)
}
