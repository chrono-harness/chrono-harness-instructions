//! Publish the rule data from a checked dev commit.
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub enum PublicationBase {
    Stale,
    First,
    Existing,
}

pub fn publication_base(current: bool, published: bool) -> (base: PublicationBase)
    ensures
        (base is Stale) == !current,
        (base is First) == (current && !published),
        (base is Existing) == (current && published),
{
    if !current { PublicationBase::Stale }
    else if published { PublicationBase::Existing }
    else { PublicationBase::First }
}

pub fn staged_change(status: i32) -> (changed: Option<bool>)
    ensures changed == (if status == 0 { Some(false) }
                        else if status == 1 { Some(true) }
                        else { None }),
{
    match status { 0 => Some(false), 1 => Some(true), _ => None }
}
}

#[cfg(not(verus_keep_ghost))]
mod host {
    use super::{PublicationBase, publication_base, staged_change};
    use std::{env, fs, io, path::Path, process::Command};

    const DEV: &str = "refs/heads/dev";
    const PUBLISHED: &str = "refs/heads/published-rules";
    pub const FILES: [&str; 3] = [
        "rules.json",
        "methodology-attribution.md",
        "trureturing-Apache-2.0.txt",
    ];

    pub struct GitResult {
        pub code: i32,
        pub stdout: String,
    }

    impl GitResult {
        fn checked(self) -> io::Result<String> {
            if self.code == 0 {
                Ok(self.stdout)
            } else {
                Err(io::Error::other(format!("git exited with {}", self.code)))
            }
        }
    }

    pub fn publish(
        source: &str,
        mut git: impl FnMut(&[&str]) -> io::Result<GitResult>,
        stage: impl FnOnce() -> io::Result<()>,
    ) -> io::Result<()> {
        let dev = git(&["ls-remote", "origin", DEV])?.checked()?;
        let current = dev.split_ascii_whitespace().next() == Some(source);
        // A stale event must not depend on the state of the publication branch.
        let published = current
            && !git(&["ls-remote", "--heads", "origin", PUBLISHED])?
                .checked()?
                .is_empty();
        match publication_base(current, published) {
            PublicationBase::Stale => return Ok(()),
            PublicationBase::First => {
                git(&["switch", "--orphan", "published-rules"])?.checked()?;
            }
            PublicationBase::Existing => {
                git(&["fetch", "--no-tags", "--depth=1", "origin", PUBLISHED])?.checked()?;
                git(&["switch", "--detach", "FETCH_HEAD"])?.checked()?;
            }
        }
        stage()?;
        git(&["add", "--", FILES[0], FILES[1], FILES[2]])?.checked()?;
        let diff = git(&["diff", "--cached", "--quiet"])?;
        match staged_change(diff.code) {
            Some(false) => return Ok(()),
            None => {
                diff.checked()?;
            }
            Some(true) => {}
        }
        git(&[
            "-c",
            "user.name=github-actions",
            "-c",
            "user.email=41898282+github-actions[bot]@users.noreply.github.com",
            "commit",
            "-m",
            &format!("Publish rules from {source}"),
        ])?
        .checked()?;
        git(&["push", "origin", "HEAD:refs/heads/published-rules"])?.checked()?;
        Ok(())
    }

    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        use std::process::Stdio;
        let source = env::var("SOURCE_SHA")?;
        let rules = chrono_instructions::dispatch(&["publish-rules".into()]);
        if rules.exit_code != 0 {
            return Err(rules.stderr.into());
        }
        let licenses = Path::new("docs/licenses");
        let attribution = fs::read(licenses.join(FILES[1]))?;
        let license = fs::read(licenses.join(FILES[2]))?;
        publish(
            &source,
            |args| {
                let result = Command::new("git")
                    .args(args)
                    .stderr(Stdio::inherit())
                    .output()?;
                Ok(GitResult {
                    code: result.status.code().unwrap_or(-1),
                    stdout: String::from_utf8(result.stdout).map_err(io::Error::other)?,
                })
            },
            || {
                fs::write(FILES[0], rules.stdout)?;
                fs::write(FILES[1], attribution)?;
                fs::write(FILES[2], license)
            },
        )?;
        Ok(())
    }
}

#[cfg(not(verus_keep_ghost))]
pub use host::{GitResult, publish};
#[cfg(not(verus_keep_ghost))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    host::run()
}
