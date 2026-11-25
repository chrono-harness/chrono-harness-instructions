use std::io::{self, Write};
use std::process::ExitCode;

mod log;

fn main() -> ExitCode {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    let logger = match log::Log::for_args(&args) {
        Ok(logger) => logger,
        Err(error) => {
            log::diagnostic("E_LOG_CONFIG", &error, None);
            return ExitCode::from(2);
        }
    };
    let mut log_error = logger
        .as_ref()
        .and_then(|writer| writer.event("cli.started", None, None).err());
    let result = chrono_instructions::dispatch(&args);
    if let Some(writer) = &logger {
        if !result.stderr.is_empty() {
            let error = writer
                .event("cli.diagnostic", Some(result.stderr.trim_end()), None)
                .err();
            log_error = log_error.or(error);
        }
        let error = writer
            .event("cli.finished", None, Some(result.exit_code))
            .err();
        log_error = log_error.or(error);
    }
    let streams_written = io::stdout()
        .lock()
        .write_all(result.stdout.as_bytes())
        .is_ok()
        && io::stderr()
            .lock()
            .write_all(result.stderr.as_bytes())
            .is_ok();
    let (exit, diagnostic) = chrono_instructions::publication_decision::finish_cli(
        streams_written,
        log_error,
        result.exit_code,
    );
    if let Some(error) = diagnostic {
        log::diagnostic("E_LOG_WRITE", &error.to_string(), Some(result.exit_code));
    }
    ExitCode::from(exit)
}
