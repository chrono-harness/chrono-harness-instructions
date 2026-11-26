//! Final event layout in the raw Markdown offsets reported by the top-level block parser.
#[cfg(verus_keep_ghost)]
use super::verified_bytes;
#[cfg(not(verus_keep_ghost))]
use crate::verified_bytes;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

/// The event may occupy the whole body or follow a blank line.
pub fn separated(prefix: &[u8]) -> (valid: bool)
    ensures valid == (prefix.len() == 0 || (prefix.len() >= 2
        && prefix@[prefix.len() - 2] == 10 && prefix@[prefix.len() - 1] == 10)),
{
    prefix.is_empty() || (prefix.len() >= 2
        && prefix[prefix.len() - 2] == 10u8 && prefix[prefix.len() - 1] == 10u8)
}

/// The final paragraph must have the expected starting byte and the complete visible text.
pub fn paragraph_matches(paragraph: Option<(usize, &[u8])>, start: usize, text: &[u8]) -> (valid: bool)
    ensures valid == match paragraph {
        Some((offset, bytes)) => offset == start && bytes@ == text@,
        None => false,
    },
{
    match paragraph {
        Some((offset, bytes)) => offset == start && verified_bytes::equal(bytes, text),
        None => false,
    }
}

/// A framed event has exactly one LF between the opening, the visible summary, and the closing.
pub fn visible_summary(paragraph: Option<(usize, &[u8])>, opening: usize, opening_len: usize,
    summary: &[u8], closing: usize) -> (valid: bool)
    ensures valid == match paragraph {
        Some((offset, bytes)) => offset == opening as int + opening_len as int + 1
            && bytes@ == summary@ && closing == offset as int + summary.len() as int + 1,
        None => false,
    },
{
    match paragraph {
        Some((offset, bytes)) => offset > opening && offset - opening - 1 == opening_len
            && verified_bytes::equal(bytes, summary) && closing > offset
            && closing - offset - 1 == summary.len(),
        None => false,
    }
}

}

#[cfg(not(verus_keep_ghost))]
use pulldown_cmark::{Event, Options, Parser, Tag};
#[cfg(not(verus_keep_ghost))]
use regex::Regex;
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize};
#[cfg(not(verus_keep_ghost))]
use std::{io::Read, sync::LazyLock};

#[cfg(not(verus_keep_ghost))]
pub const PATTERN: &str = include_str!("../../../assets/instructions/github-event-v4.regex");
#[cfg(not(verus_keep_ghost))]
const V3_PATTERN: &str = include_str!("../../../assets/instructions/github-event-v3.regex");
#[cfg(not(verus_keep_ghost))]
const LEGACY_PATTERN: &str = include_str!("../../../assets/instructions/github-event-v2.regex");
#[cfg(not(verus_keep_ghost))]
const LEGACY_BEGIN: &str = "CHRONO_AGENT_BEGIN_082981f29eb429599bac65802d2f5465";
#[cfg(not(verus_keep_ghost))]
const LEGACY_END: &str = "CHRONO_AGENT_END_94f7082e08249c1b16af162e9625946a";
#[cfg(not(verus_keep_ghost))]
static LEGACY_LINE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(LEGACY_PATTERN.trim_end()).unwrap());
#[cfg(not(verus_keep_ghost))]
fn frame_expression(pattern: &str) -> Regex {
    // Rust regex has no backreferences. Compare the captured closing ID in identity().
    Regex::new(&pattern.trim_end().replace(r"\1", "([0-9a-f-]+)")).unwrap()
}
#[cfg(not(verus_keep_ghost))]
static FRAME: LazyLock<Regex> = LazyLock::new(|| frame_expression(PATTERN));
#[cfg(not(verus_keep_ghost))]
static V3_FRAME: LazyLock<Regex> = LazyLock::new(|| frame_expression(V3_PATTERN));

#[cfg(not(verus_keep_ghost))]
fn unknown_model() -> String {
    "unknown".into()
}

#[cfg(not(verus_keep_ghost))]
#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AgentEvent {
    pub agent: String,
    pub session: String,
    #[serde(default = "unknown_model")]
    pub model: String,
    pub parent: String,
    pub task: String,
    pub event: String,
    pub action: String,
    pub state: String,
    pub target: String,
}

#[cfg(not(verus_keep_ghost))]
impl AgentEvent {
    fn fields(
        [agent, session, parent, task, event, action, state, target]: [&str; 8],
        model: &str,
        closing: Option<&str>,
    ) -> Result<Self, String> {
        use crate::event_identity::{Identity, identity};
        match identity(
            session.as_bytes(),
            parent.as_bytes(),
            task.as_bytes(),
            event.as_bytes(),
            closing.map(str::as_bytes),
        ) {
            Identity::Valid => (),
            Identity::ClosingMismatch => {
                return Err("unknown: opening and closing event IDs differ".into());
            }
            Identity::ReusedId => {
                return Err(
                    "unknown: session, parent, task and event require independent IDs".into(),
                );
            }
        }
        Ok(Self {
            agent: agent.into(),
            session: session.into(),
            model: model.into(),
            parent: parent.into(),
            task: task.into(),
            event: event.into(),
            action: action.into(),
            state: state.into(),
            target: target.into(),
        })
    }

    fn framed(frame: &str) -> Result<Self, String> {
        let [
            event,
            parent,
            task,
            agent,
            session,
            model,
            action,
            state,
            target,
            closing,
        ] = if crate::event_identity::uses_v3_frame(frame.as_bytes()) {
            let captures = V3_FRAME
                .captures(frame)
                .ok_or("unknown: malformed v3 frame")?;
            let [
                event,
                session,
                parent,
                task,
                agent,
                action,
                state,
                target,
                closing,
            ] = captures.extract().1;
            [
                event, parent, task, agent, session, "unknown", action, state, target, closing,
            ]
        } else {
            FRAME
                .captures(frame)
                .ok_or("unknown: malformed v4 frame")?
                .extract()
                .1
        };
        Self::fields(
            [agent, session, parent, task, event, action, state, target],
            model,
            Some(closing),
        )
    }
}

#[cfg(not(verus_keep_ghost))]
enum Marker {
    Opening(usize),
    Closing(usize),
    Legacy(usize),
}

#[cfg(not(verus_keep_ghost))]
pub fn parse(body: &str) -> Result<AgentEvent, String> {
    let normalized = body.replace("\r\n", "\n");
    if normalized.contains('\r') {
        return Err("unknown: bare carriage return".into());
    }
    let body = normalized.strip_suffix('\n').unwrap_or(&normalized);
    let opts = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_GFM
        | Options::ENABLE_FOOTNOTES;
    let mut depth = 0;
    let mut markers = Vec::new();
    let mut last_paragraph = None;
    for (token, range) in Parser::new_ext(body, opts).into_offset_iter() {
        match token {
            Event::Start(tag) => {
                if depth == 0 {
                    let text = body[range.clone()].trim_end_matches('\n');
                    match tag {
                        Tag::HtmlBlock if text.starts_with("<!-- chrono-agent:") => {
                            markers.push(Marker::Opening(range.start))
                        }
                        Tag::HtmlBlock if text.starts_with("<!-- /chrono-agent:") => {
                            markers.push(Marker::Closing(range.start))
                        }
                        Tag::Paragraph => {
                            last_paragraph = Some((range.start, text.as_bytes()));
                            for row in text.lines() {
                                if row.starts_with(LEGACY_BEGIN) || row.ends_with(LEGACY_END) {
                                    markers.push(Marker::Legacy(range.start));
                                }
                            }
                        }
                        _ => (),
                    }
                }
                depth += 1;
            }
            Event::End(_) => depth -= 1,
            _ => (),
        }
    }
    let (offset, event) = match markers.as_slice() {
        [Marker::Legacy(start)]
            if paragraph_matches(last_paragraph, *start, body[*start..].as_bytes()) =>
        {
            let c = LEGACY_LINE
                .captures(&body[*start..])
                .ok_or("unknown: malformed v2 event")?;
            (*start, AgentEvent::fields(c.extract().1, "unknown", None)?)
        }
        [Marker::Opening(start), Marker::Closing(end)] => {
            let frame = &body[*start..];
            let (opening, rest) = frame
                .split_once('\n')
                .ok_or("unknown: missing event summary")?;
            let (summary, _) = rest
                .split_once('\n')
                .ok_or("unknown: missing closing marker")?;
            if !visible_summary(
                last_paragraph,
                *start,
                opening.len(),
                summary.as_bytes(),
                *end,
            ) {
                return Err("unknown: expected a top-level visible event summary".into());
            }
            (*start, AgentEvent::framed(frame)?)
        }
        _ => return Err("unknown: expected one final top-level event".into()),
    };
    if !separated(&body.as_bytes()[..offset]) {
        return Err("unknown: missing blank separator".into());
    }
    Ok(event)
}

#[cfg(not(verus_keep_ghost))]
pub fn format(event: &AgentEvent) -> Result<String, String> {
    let frame = format!(
        "<!-- chrono-agent:v4 event={id} parent={} task={} -->\n**Agent** `{}` · **session** `{}` · **model** `{}` · **action** `{}` · **state** `{}` · **target** `{}`\n<!-- /chrono-agent:v4 event={id} -->",
        event.parent,
        event.task,
        event.agent,
        event.session,
        event.model,
        event.action,
        event.state,
        event.target,
        id = event.event
    );
    if parse(&frame)? != *event {
        return Err("invalid event fields".into());
    }
    Ok(frame)
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn command(op: &str, input: impl Read) -> Result<String, String> {
    if op == "event-regex" {
        return Ok(PATTERN.into());
    }
    let mut text = String::new();
    let mut input = input;
    input.read_to_string(&mut text).map_err(|e| e.to_string())?;
    let result = if op == "event-read" {
        serde_json::to_string(&parse(&text)?).map_err(|e| e.to_string())?
    } else {
        format(&serde_json::from_str(&text).map_err(|e| e.to_string())?)?
    };
    Ok(format!("{result}\n"))
}
