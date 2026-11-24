//! Public GitHub event transport. Raw source ranges distinguish standalone paragraphs from Markdown examples.
use pulldown_cmark::{Event, Options, Parser, Tag};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{io::Read, sync::LazyLock};

pub const PATTERN: &str = include_str!("../../../assets/instructions/github-event-v2.regex");
const BEGIN: &str = "CHRONO_AGENT_BEGIN_082981f29eb429599bac65802d2f5465";
const END: &str = "CHRONO_AGENT_END_94f7082e08249c1b16af162e9625946a";
static LINE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(PATTERN.trim_end_matches('\n')).unwrap());

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

fn fields(line: &str) -> Result<AgentEvent, String> {
    let captures = LINE.captures(line).ok_or("unknown: malformed v2 event")?;
    let [agent, session, parent, task, event, action, state, target] = captures.extract().1;
    let ids = [session, parent, task, event];
    for (i, id) in ids.iter().enumerate() {
        if ids[..i].contains(id) {
            return Err("unknown: session, parent, task and event require independent IDs".into());
        }
    }
    Ok(AgentEvent {
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

pub fn parse(body: &str) -> Result<AgentEvent, String> {
    let normalized = body.replace("\r\n", "\n");
    if normalized.contains('\r') {
        return Err("unknown: bare carriage return".into());
    }
    let body = normalized.strip_suffix('\n').unwrap_or(&normalized);
    let offset = body.rfind('\n').map_or(0, |i| i + 1);
    let line = &body[offset..];
    let event = fields(line)?;
    if offset != 0 && !body[..offset].ends_with("\n\n") {
        return Err("unknown: missing blank separator".into());
    }
    let opts = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_GFM
        | Options::ENABLE_FOOTNOTES;
    let mut depth = 0;
    let mut signatures = 0;
    let mut terminal = false;
    for (token, range) in Parser::new_ext(body, opts).into_offset_iter() {
        match token {
            Event::Start(tag) => {
                if depth == 0 && matches!(tag, Tag::Paragraph) {
                    let para = body[range.clone()].trim_end_matches('\n');
                    for row in para.lines() {
                        if row.starts_with(BEGIN) || row.ends_with(END) {
                            signatures += 1;
                        }
                    }
                    if range.start == offset && para == line {
                        terminal = true;
                    }
                }
                depth += 1;
            }
            Event::End(_) => depth -= 1,
            _ => (),
        }
    }
    if !terminal || signatures != 1 {
        return Err("unknown: expected one final top-level event paragraph".into());
    }
    Ok(event)
}

pub fn format(event: &AgentEvent) -> Result<String, String> {
    let line = format!(
        "{BEGIN} v=2 agent={} session={} parent={} task={} event={} action={} state={} target={} {END}",
        event.agent,
        event.session,
        event.parent,
        event.task,
        event.event,
        event.action,
        event.state,
        event.target
    );
    if parse(&line)? != *event {
        return Err("invalid event fields".into());
    }
    Ok(line)
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
