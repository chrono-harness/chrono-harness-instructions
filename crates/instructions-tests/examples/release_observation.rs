//! Release acceptance for the generated guide symlink and preserved host line.
#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub fn expected_target() -> (value: &'static str)
    ensures value@ == "CLAUDE.md"@,
{ "CLAUDE.md" }

pub fn expected_line() -> (value: &'static str)
    ensures value@ == "Local setup."@,
{ "Local setup." }

pub fn accepts(target_matches: bool, original_line: bool) -> (accepted: bool)
    ensures accepted == (target_matches && original_line),
{
    target_matches && original_line
}
}

#[cfg(not(verus_keep_ghost))]
fn grep(mode: &str, pattern: &str, path: &str) -> Result<bool, String> {
    // grep interprets its documented matching mode and reports the original matching lines.
    std::process::Command::new("grep")
        .args([mode, pattern, path])
        .status()
        .map(|result| result.success())
        .map_err(|error| error.to_string())
}

#[cfg(not(verus_keep_ghost))]
fn main() -> Result<(), String> {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let [target, guide] = arguments.as_slice() else {
        return Err("usage: release_observation TARGET GUIDE".into());
    };
    let target_matches = target == expected_target();
    let original_line = grep("-Fx", expected_line(), guide)?;
    let admitted = accepts(target_matches, original_line);
    if admitted {
        Ok(())
    } else {
        Err(format!("unexpected release observation for {arguments:?}"))
    }
}
