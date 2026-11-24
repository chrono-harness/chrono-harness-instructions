use chrono_instructions::github_event::{AgentEvent, format, parse};

fn sample() -> AgentEvent {
    serde_json::from_str(r#"{"agent":"codex/root/sub_task","session":"058ca5a5-1923-4ec4-8cf6-74a1694a510d","parent":"bf49786f-66b2-4f31-929b-1be2b859c64d","task":"a1000000-0000-4000-8000-000000000000","event":"e1000000-0000-4000-b000-000000000000","action":"ci.wait","state":"waiting","target":"org/repo/run/12/attempt/2"}"#).unwrap()
}
#[test]
fn roundtrip_raw_body_with_optional_single_terminator() {
    let event = sample();
    let line = format(&event).unwrap();
    for separator in ["\n", "\r\n"] {
        for ending in ["", separator] {
            assert_eq!(
                parse(&format!("done{separator}{separator}{line}{ending}")).unwrap(),
                event
            );
        }
    }
    assert_eq!(parse(&line).unwrap(), event);
}
#[test]
fn ignores_markdown_examples_and_rejects_ambiguous_or_nonterminal_frames() {
    let event = sample();
    let line = format(&event).unwrap();
    for sample in [
        format!("> - `{line}`"),
        format!("> > {line}"),
        format!("~~~\n{line}\n~~~"),
        format!("    {line}"),
        format!("<!--\n{line}\n-->"),
    ] {
        assert!(parse(&sample).is_err(), "{sample}");
        assert_eq!(parse(&format!("{sample}\n\n{line}")).unwrap(), event);
    }
    for invalid in [
        format!("message\n{line}"),
        format!("```\n\n{line}"),
        format!("<!--\n\n{line}"),
        format!("{line}\n\n{line}"),
        format!("{line}\nmore"),
        format!("{line}\n\n"),
        format!("{line} "),
        format!(" {line}"),
        line.replace(" v=2 ", " v=２ "),
        line.replace("agent=", "аgent="),
        line.replace("action=ci.wait", "action=ciXwait"),
        line.replace(" 4", " 7"),
        line.replace(
            "parent=bf49786f-66b2-4f31-929b-1be2b859c64d",
            "parent=058ca5a5-1923-4ec4-8cf6-74a1694a510d",
        ),
    ] {
        if invalid != line {
            assert!(parse(&invalid).is_err(), "{invalid}");
        }
    }
}
#[test]
fn validates_targets_ids_and_emitted_fields() {
    let mut e = sample();
    for target in [
        "self",
        "task/a1000000-0000-4000-8000-000000000000",
        "org/repo#1",
        "org/repo/tag/v0.1.0-alpha.4",
        "org/repo@0123456789012345678901234567890123456789",
        "org/repo/run/1",
    ] {
        e.target = target.into();
        assert_eq!(parse(&format(&e).unwrap()).unwrap(), e);
    }
    for target in [
        "org/repo#0",
        "org/repo/run/1/attempt/0",
        "self\n",
        "https://example.org",
        "org/repo/tag/无",
    ] {
        e.target = target.into();
        assert!(format(&e).is_err());
    }
    e = sample();
    e.session = e.session.replace("4ec4", "7ec4");
    assert!(format(&e).is_err());
    e = sample();
    e.event = e.task.clone();
    assert!(format(&e).is_err());
}
