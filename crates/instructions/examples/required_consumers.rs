//! Check the results emitted by the unfiltered libtest run.
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[path = "../src/verified_bytes.rs"]
mod verified_bytes;

verus! {
pub fn has_line(lines: &[&[u8]], expected: &[u8]) -> (found: bool)
    ensures found == exists|j: int| 0 <= j < lines.len() && lines@[j]@ == expected@,
{
    let mut i = 0;
    while i < lines.len()
        invariant i <= lines.len(), forall|j: int| 0 <= j < i ==> lines@[j]@ != expected@,
        decreases lines.len() - i,
    {
        if verified_bytes::equal(lines[i], expected) { return true; }
        i += 1;
    }
    false
}
}

#[cfg(not(verus_keep_ghost))]
fn main() -> std::io::Result<()> {
    use std::io;
    let mut args = std::env::args_os().skip(1);
    let path = args
        .next()
        .ok_or_else(|| io::Error::other("expected libtest output path"))?;
    let bytes = std::fs::read(path)?;
    let lines: Vec<_> = bytes.split(|&b| b == b'\n').collect();
    if args.len() == 0 {
        return Err(io::Error::other("expected named consumers"));
    }
    for name in args {
        let name = name
            .into_string()
            .map_err(|_| io::Error::other("non-UTF-8 test name"))?;
        if !has_line(&lines, format!("test {name} ... ok").as_bytes()) {
            return Err(io::Error::other(format!(
                "E_VERUS_COVERAGE: missing passed test {name}"
            )));
        }
    }
    Ok(())
}
