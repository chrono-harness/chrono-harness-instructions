//! Extract the final event frame from the raw GitHub Markdown using top-level block ranges.
use pulldown_cmark::{Event, Options, Parser, Tag};
use regex::{Captures, Regex};
use serde::{Deserialize, Serialize};
use std::{io::Read, sync::LazyLock};

pub const PATTERN: &str = include_str!("../../../assets/instructions/github-event-v3.regex");
const LEGACY_PATTERN: &str = include_str!("../../../assets/instructions/github-event-v2.regex");
const LEGACY_BEGIN: &str = "CHRONO_AGENT_BEGIN_082981f29eb429599bac65802d2f5465";
const LEGACY_END: &str = "CHRONO_AGENT_END_94f7082e08249c1b16af162e9625946a";
static LEGACY_LINE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(LEGACY_PATTERN.trim_end()).unwrap());
static FRAME: LazyLock<Regex> = LazyLock::new(|| {
    // Rust regex has no backreferences. Capture the closing spelling and compare it after matching.
    Regex::new(&PATTERN.trim_end().replace(r"\1", "([0-9a-f-]+)")).unwrap()
});

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AgentEvent {
    pub agent: String,
    pub session: String,
    pub parent: String,
    pub task: String,
    pub event: String,
    pub action: String,
    pub state: String,
    pub target: String,
}

impl AgentEvent {
    fn fields(
        [agent, session, parent, task, event, action, state, target]: [&str; 8],
    ) -> Result<Self, String> {
        let ids = [session, parent, task, event];
        if ids.iter().enumerate().any(|(i, id)| ids[..i].contains(id)) {
            return Err("unknown: session, parent, task and event require independent IDs".into());
        }
        Ok(Self {
            agent: agent.into(),
            session: session.into(),
            parent: parent.into(),
            task: task.into(),
            event: event.into(),
            action: action.into(),
            state: state.into(),
            target: target.into(),
        })
    }

    fn framed(c: Captures<'_>) -> Result<Self, String> {
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
        ] = c.extract().1;
        if event != closing {
            return Err("unknown: opening and closing event IDs differ".into());
        }
        Self::fields([agent, session, parent, task, event, action, state, target])
    }
}

enum Marker {
    Opening(usize),
    Closing(usize),
    Legacy(usize),
}

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
                            last_paragraph = Some((range.start, text));
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
        [Marker::Legacy(start)] if last_paragraph == Some((*start, &body[*start..])) => {
            let c = LEGACY_LINE
                .captures(&body[*start..])
                .ok_or("unknown: malformed v2 event")?;
            (*start, AgentEvent::fields(c.extract().1)?)
        }
        [Marker::Opening(start), Marker::Closing(end)] => {
            let frame = &body[*start..];
            let c = FRAME.captures(frame).ok_or("unknown: malformed v3 frame")?;
            let (opening, rest) = frame
                .split_once('\n')
                .ok_or("unknown: missing event summary")?;
            let (summary, _) = rest
                .split_once('\n')
                .ok_or("unknown: missing closing marker")?;
            let middle = start + opening.len() + 1;
            if last_paragraph != Some((middle, summary)) || *end != middle + summary.len() + 1 {
                return Err("unknown: expected a top-level visible event summary".into());
            }
            (*start, AgentEvent::framed(c)?)
        }
        _ => return Err("unknown: expected one final top-level event".into()),
    };
    if offset != 0 && !body[..offset].ends_with("\n\n") {
        return Err("unknown: missing blank separator".into());
    }
    Ok(event)
}

pub fn format(event: &AgentEvent) -> Result<String, String> {
    let frame = format!(
        "<!-- chrono-agent:v3 event={id} session={} parent={} task={} -->\n**Agent** `{}` · **action** `{}` · **state** `{}` · **target** `{}`\n<!-- /chrono-agent:v3 event={id} -->",
        event.session,
        event.parent,
        event.task,
        event.agent,
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
