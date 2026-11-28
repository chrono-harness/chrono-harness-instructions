//! Identity relationships among the byte slices captured from a GitHub work event.
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;

#[cfg(verus_keep_ghost)]
#[path = "verified_bytes.rs"]
mod verified_bytes;
#[cfg(not(verus_keep_ghost))]
use crate::verified_bytes;

#[cfg(verus_keep_ghost)]
#[path = "github_event.rs"]
pub mod layout;

verus! {

/// Select the legacy framed grammar from the start of the original Markdown bytes.
pub fn uses_v3_frame(frame: &[u8]) -> (legacy: bool)
    ensures ({
        let marker = "<!-- chrono-agent:v3 ".spec_bytes();
        legacy == (frame.len() >= marker.len() && frame@.subrange(0, marker.len() as int) == marker)
    }),
{
    let prefix = "<!-- chrono-agent:v3 ".as_bytes();
    if frame.len() < prefix.len() { return false; }
    verified_bytes::equal(&frame[0..prefix.len()], prefix)
}

pub enum Identity {
    Valid,
    ClosingMismatch,
    ReusedId,
}

/// A framed closing ID repeats the opening event. The four identity roles must be independent.
pub fn identity(session: &[u8], parent: &[u8], task: &[u8], event: &[u8], closing: Option<&[u8]>)
    -> (verdict: Identity)
    ensures
        (verdict is ClosingMismatch) == (match closing { Some(end) => event@ != end@, None => false }),
        (verdict is ReusedId) == (!(verdict is ClosingMismatch) &&
            (session@ == parent@ || session@ == task@ || session@ == event@ ||
             parent@ == task@ || parent@ == event@ || task@ == event@)),
        (verdict is Valid) == (!(verdict is ClosingMismatch) && !(verdict is ReusedId)),
{
    if let Some(end) = closing {
        if !verified_bytes::equal(event, end) { return Identity::ClosingMismatch; }
    }
    if verified_bytes::equal(session, parent) || verified_bytes::equal(session, task) || verified_bytes::equal(session, event)
        || verified_bytes::equal(parent, task) || verified_bytes::equal(parent, event) || verified_bytes::equal(task, event) {
        return Identity::ReusedId;
    }
    Identity::Valid
}

#[derive(Clone, Copy)]
pub enum Version {
    V2,
    V3,
    V4,
}

#[cfg(verus_keep_ghost)]
pub open spec fn malformed(version: Version) -> &'static str {
    match version {
        Version::V2 => "unknown: malformed v2 event",
        Version::V3 => "unknown: malformed v3 frame",
        Version::V4 => "unknown: malformed v4 frame",
    }
}

pub struct CapturedFields<'a> {
    pub fields: [&'a str; 8],
    pub model: &'a str,
    pub closing: Option<&'a str>,
}

/// The published regex supplies lexical captures. Decide whether they describe one event.
pub fn read_fields<'a>(version: Version, capture: Option<CapturedFields<'a>>)
    -> (result: Result<CapturedFields<'a>, &'static str>)
    ensures result == match capture {
        None => Err(malformed(version)),
        Some(fields) => {
            let ids = fields.fields@;
            let session = ids[1].spec_bytes();
            let parent = ids[2].spec_bytes();
            let task = ids[3].spec_bytes();
            let event = ids[4].spec_bytes();
            if match fields.closing { Some(end) => end.spec_bytes() != event, None => false } {
                Err("unknown: opening and closing event IDs differ")
            } else if session == parent || session == task || session == event
                || parent == task || parent == event || task == event
            {
                Err("unknown: session, parent, task and event require independent IDs")
            } else {
                Ok(fields)
            }
        }
    },
{
    let fields = match capture {
        Some(fields) => fields,
        None => return Err(match version {
            Version::V2 => "unknown: malformed v2 event",
            Version::V3 => "unknown: malformed v3 frame",
            Version::V4 => "unknown: malformed v4 frame",
        }),
    };
    let closing = match fields.closing {
        Some(end) => Some(end.as_bytes()),
        None => None,
    };
    match identity(fields.fields[1].as_bytes(), fields.fields[2].as_bytes(),
        fields.fields[3].as_bytes(), fields.fields[4].as_bytes(), closing)
    {
        Identity::Valid => Ok(fields),
        Identity::ClosingMismatch => Err("unknown: opening and closing event IDs differ"),
        Identity::ReusedId => Err("unknown: session, parent, task and event require independent IDs"),
    }
}

}
