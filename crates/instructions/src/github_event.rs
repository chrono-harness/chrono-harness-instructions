//! Final event layout in the raw Markdown offsets reported by the top-level block parser.
#[cfg(verus_keep_ghost)]
use super::verified_bytes;
#[cfg(not(verus_keep_ghost))]
use crate::verified_bytes;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;

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


#[derive(Clone, Copy)]
pub enum Marker {
    Opening(usize),
    Closing(usize),
    Legacy(usize),
}

pub(crate) const LEGACY_BEGIN: &'static str = "CHRONO_AGENT_BEGIN_082981f29eb429599bac65802d2f5465";
pub(crate) const LEGACY_END: &'static str = "CHRONO_AGENT_END_94f7082e08249c1b16af162e9625946a";

fn prefix(text: &[u8], marker: &[u8]) -> (found: bool)
    ensures found == (text.len() >= marker.len() &&
        text@.subrange(0, marker.len() as int) == marker@),
{
    text.len() >= marker.len() && verified_bytes::equal(&text[0..marker.len()], marker)
}

fn suffix(text: &[u8], marker: &[u8]) -> (found: bool)
    ensures found == (text.len() >= marker.len() &&
        text@.subrange(text.len() - marker.len(), text.len() as int) == marker@),
{
    text.len() >= marker.len() && verified_bytes::equal(&text[text.len() - marker.len()..text.len()], marker)
}

/// Only tokens at the document root can contribute evidence.
pub fn is_top_level(depth: usize) -> (top: bool)
    ensures top == (depth == 0),
{
    depth == 0
}

/// The tokenizer classifies the block. These byte prefixes identify candidate delimiters.
pub fn html_marker(start: usize, text: &[u8]) -> (marker: Option<Marker>)
    ensures ({
        let open = "<!-- chrono-agent:".spec_bytes();
        let close = "<!-- /chrono-agent:".spec_bytes();
        marker == if text.len() >= open.len() && text@.subrange(0, open.len() as int) == open {
            Some(Marker::Opening(start))
        } else if text.len() >= close.len() && text@.subrange(0, close.len() as int) == close {
            Some(Marker::Closing(start))
        } else { None }
    }),
{
    if prefix(text, "<!-- chrono-agent:".as_bytes()) {
        Some(Marker::Opening(start))
    } else if prefix(text, "<!-- /chrono-agent:".as_bytes()) {
        Some(Marker::Closing(start))
    } else {
        None
    }
}

pub(crate) fn legacy_marker(start: usize, row: &[u8]) -> (marker: Option<Marker>)
    ensures ({
        let open = LEGACY_BEGIN.spec_bytes();
        let close = LEGACY_END.spec_bytes();
        marker == if (row.len() >= open.len() && row@.subrange(0, open.len() as int) == open)
            || (row.len() >= close.len() &&
                row@.subrange(row.len() - close.len(), row.len() as int) == close)
        { Some(Marker::Legacy(start)) } else { None }
    }),
{
    if prefix(row, LEGACY_BEGIN.as_bytes()) || suffix(row, LEGACY_END.as_bytes()) {
        Some(Marker::Legacy(start))
    } else {
        None
    }
}

/// Accept at most one terminator after CRLF normalization.
pub fn body_end(body: &[u8]) -> (end: Option<usize>)
    ensures end == if exists|i: int| 0 <= i < body.len() && body@[i] == 13 {
        None
    } else if body.len() > 0 && body@[body.len() - 1] == 10 {
        Some((body.len() - 1) as usize)
    } else {
        Some(body.len())
    },
{
    let mut i = 0;
    while i < body.len()
        invariant i <= body.len(), forall|j: int| 0 <= j < i ==> body@[j] != 13,
        decreases body.len() - i,
    {
        if body[i] == 13u8 {
            return None;
        }
        i += 1;
    }
    if i > 0 && body[i - 1] == 10u8 { Some(i - 1) } else { Some(i) }
}

pub enum Frame {
    Legacy(usize),
    Framed(usize, usize),
    MissingOrAmbiguous,
}

#[cfg(verus_keep_ghost)]
pub open spec fn final_frame(markers: Seq<Marker>) -> Frame {
    if markers.len() == 1 {
        match markers[0] {
            Marker::Legacy(start) => Frame::Legacy(start),
            _ => Frame::MissingOrAmbiguous,
        }
    } else if markers.len() == 2 {
        match (markers[0], markers[1]) {
            (Marker::Opening(start), Marker::Closing(end)) => Frame::Framed(start, end),
            _ => Frame::MissingOrAmbiguous,
        }
    } else {
        Frame::MissingOrAmbiguous
    }
}

/// Reduce the complete list of top-level event markers in Markdown order.
pub fn select_frame(markers: &[Marker]) -> (frame: Frame)
    ensures frame == final_frame(markers@),
{
    if markers.len() == 1 {
        if let Marker::Legacy(start) = markers[0] {
            return Frame::Legacy(start);
        }
    } else if markers.len() == 2 {
        if let (Marker::Opening(start), Marker::Closing(end)) = (markers[0], markers[1]) {
            return Frame::Framed(start, end);
        }
    }
    Frame::MissingOrAmbiguous
}

pub enum FrameLayout {
    Visible,
    MissingSummary,
    MissingClosing,
    HiddenOrMisplaced,
}

/// Preserve the first layout failure recorded from the raw line boundaries.
pub fn framed_layout(paragraph: Option<(usize, &[u8])>, opening: usize, closing: usize,
    lines: Option<(usize, Option<&[u8]>)>) -> (layout: FrameLayout)
    ensures layout == match lines {
        None => FrameLayout::MissingSummary,
        Some((_, None)) => FrameLayout::MissingClosing,
        Some((opening_len, Some(summary))) => match paragraph {
            Some((offset, bytes)) => if offset == opening as int + opening_len as int + 1
                && bytes@ == summary@ && closing == offset as int + summary.len() as int + 1
            { FrameLayout::Visible } else { FrameLayout::HiddenOrMisplaced },
            None => FrameLayout::HiddenOrMisplaced,
        },
    },
{
    match lines {
        None => FrameLayout::MissingSummary,
        Some((_, None)) => FrameLayout::MissingClosing,
        Some((opening_len, Some(summary))) => {
            if visible_summary(paragraph, opening, opening_len, summary, closing) {
                FrameLayout::Visible
            } else {
                FrameLayout::HiddenOrMisplaced
            }
        },
    }
}

/// Compare every field after parsing the bytes returned to the caller.
pub fn same_fields(left: &[&[u8]; 9], right: &[&[u8]; 9]) -> (same: bool)
    ensures same == (forall|i: int| 0 <= i < 9 ==> (#[trigger] left@[i])@ == right@[i]@),
{
    let mut i = 0;
    while i < 9
        invariant
            i <= 9,
            forall|j: int| 0 <= j < i ==> (#[trigger] left@[j])@ == right@[j]@,
        decreases 9 - i,
    {
        if !verified_bytes::equal(left[i], right[i]) {
            return false;
        }
        i += 1;
    }
    true
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
pub(crate) const LEGACY_PATTERN: &str =
    include_str!("../../../assets/instructions/github-event-v2.regex");
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
    fn from_capture(version: Version, capture: Option<CapturedFields<'_>>) -> Result<Self, String> {
        let CapturedFields {
            fields: [agent, session, parent, task, event, action, state, target],
            model,
            ..
        } = read_fields(version, capture)?;
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

    fn field_bytes(&self) -> [&[u8]; 9] {
        [
            &self.agent,
            &self.session,
            &self.parent,
            &self.task,
            &self.event,
            &self.action,
            &self.state,
            &self.target,
            &self.model,
        ]
        .map(|field| field.as_bytes())
    }

    fn framed(frame: &str) -> Result<Self, String> {
        if crate::event_identity::uses_v3_frame(frame.as_bytes()) {
            Self::from_capture(
                Version::V3,
                V3_FRAME.captures(frame).map(|captures| {
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
                    CapturedFields {
                        fields: [agent, session, parent, task, event, action, state, target],
                        model: "unknown",
                        closing: Some(closing),
                    }
                }),
            )
        } else {
            Self::from_capture(
                Version::V4,
                FRAME.captures(frame).map(|captures| {
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
                    ] = captures.extract().1;
                    CapturedFields {
                        fields: [agent, session, parent, task, event, action, state, target],
                        model,
                        closing: Some(closing),
                    }
                }),
            )
        }
    }
}

#[cfg(not(verus_keep_ghost))]
use crate::event_identity::{CapturedFields, Version, read_fields};

#[cfg(not(verus_keep_ghost))]
pub fn parse(body: &str) -> Result<AgentEvent, String> {
    let normalized = body.replace("\r\n", "\n");
    let end = body_end(normalized.as_bytes()).ok_or("unknown: bare carriage return")?;
    let body = &normalized[..end];
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
                if is_top_level(depth) {
                    let text = body[range.clone()].trim_end_matches('\n');
                    match tag {
                        Tag::HtmlBlock => markers.extend(html_marker(range.start, text.as_bytes())),
                        Tag::Paragraph => {
                            last_paragraph = Some((range.start, text.as_bytes()));
                            markers.extend(
                                text.lines()
                                    .filter_map(|row| legacy_marker(range.start, row.as_bytes())),
                            );
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
    let (offset, event) = match select_frame(&markers) {
        Frame::Legacy(start)
            if paragraph_matches(last_paragraph, start, body[start..].as_bytes()) =>
        {
            let capture = LEGACY_LINE
                .captures(&body[start..])
                .map(|captures| CapturedFields {
                    fields: captures.extract().1,
                    model: "unknown",
                    closing: None,
                });
            (start, AgentEvent::from_capture(Version::V2, capture)?)
        }
        Frame::Framed(start, end) => {
            let frame = &body[start..];
            let lines = frame.split_once('\n').map(|(opening, rest)| {
                (
                    opening.len(),
                    rest.split_once('\n').map(|(summary, _)| summary.as_bytes()),
                )
            });
            match framed_layout(last_paragraph, start, end, lines) {
                FrameLayout::Visible => (),
                FrameLayout::MissingSummary => return Err("unknown: missing event summary".into()),
                FrameLayout::MissingClosing => return Err("unknown: missing closing marker".into()),
                FrameLayout::HiddenOrMisplaced => {
                    return Err("unknown: expected a top-level visible event summary".into());
                }
            }
            (start, AgentEvent::framed(frame)?)
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
    if !same_fields(&parse(&frame)?.field_bytes(), &event.field_bytes()) {
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
