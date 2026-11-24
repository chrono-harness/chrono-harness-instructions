use chrono_instructions::github_event::{AgentEvent, format, parse};

fn sample() -> AgentEvent {
    serde_json::from_str(r#"{"agent":"codex/root/sub_task","session":"058ca5a5-1923-4ec4-8cf6-74a1694a510d","parent":"bf49786f-66b2-4f31-929b-1be2b859c64d","task":"a1000000-0000-4000-8000-000000000000","event":"e1000000-0000-4000-b000-000000000000","action":"ci.wait","state":"waiting","target":"org/repo/run/12/attempt/2"}"#).unwrap()
}
#[test]
fn roundtrip_raw_body_with_optional_single_terminator() {
    let event = sample();
    let line = format(&event).unwrap();
    for separator in ["\n", "\r\n"] {
        let line = line.replace('\n', separator);
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
    let quote = line
        .lines()
        .map(|row| format!("> > {row}"))
        .collect::<Vec<_>>()
        .join("\n");
    let indented = line
        .lines()
        .map(|row| format!("    {row}"))
        .collect::<Vec<_>>()
        .join("\n");
    for sample in [
        quote,
        indented,
        format!("~~~\n{line}\n~~~"),
        format!("<pre>\n{line}\n</pre>"),
    ] {
        assert!(parse(&sample).is_err(), "{sample}");
        assert_eq!(
            parse(&format!("{sample}\n\n{line}")).unwrap(),
            event,
            "{sample}"
        );
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
        line.replace(":v3 ", ":v4 "),
        line.replace("**Agent**", "**Аgent**"),
        line.replace("ci.wait", "ciXwait"),
        line.replace("**state**", "**State**"),
        line.replace(" · ", " | "),
        line.replace(
            "<!-- /chrono-agent:v3 event=e1",
            "<!-- /chrono-agent:v3 event=a1",
        ),
        line.replacen("e1000000", "f1000000", 1),
        line.split_once('\n').unwrap().1.to_string(),
        line.rsplit_once('\n').unwrap().0.to_string(),
        format!("<!-- chrono-agent:v4 bad -->\n\n{line}"),
        format!("{line}\rX"),
        format!(
            "- one\n\n{}",
            line.lines()
                .map(|row| format!("  {row}"))
                .collect::<Vec<_>>()
                .join("\n")
        ),
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

#[test]
fn public_v2_events_remain_readable_and_mixed_records_are_ambiguous() {
    let e = sample();
    let old = format!(
        "CHRONO_AGENT_BEGIN_082981f29eb429599bac65802d2f5465 v=2 agent={} session={} parent={} task={} event={} action={} state={} target={} CHRONO_AGENT_END_94f7082e08249c1b16af162e9625946a",
        e.agent, e.session, e.parent, e.task, e.event, e.action, e.state, e.target
    );
    assert_eq!(
        parse(&format!("existing report\r\n\r\n{old}\r\n")).unwrap(),
        e
    );
    let new = format(&e).unwrap();
    assert!(parse(&format!("{old}\n\n{new}")).is_err());
    assert!(parse(&format!("{new}\n\n{old}")).is_err());
}

#[test]
fn raw_event_reader_checks_every_captured_identity() {
    let sample = sample();
    let wire = format(&sample).unwrap();
    let ids = [&sample.session, &sample.parent, &sample.task, &sample.event];
    for (i, a) in ids.iter().enumerate() {
        for b in &ids[i + 1..] {
            let collision = wire.replace(b.as_str(), a.as_str());
            let message = parse(&collision).unwrap_err();
            assert!(message.contains("independent IDs"), "{message}");
        }
    }
    let wrong_end = wire.replace(
        &std::format!("<!-- /chrono-agent:v3 event={}", sample.event),
        &std::format!("<!-- /chrono-agent:v3 event={}", sample.task),
    );
    assert!(
        parse(&wrong_end)
            .unwrap_err()
            .contains("opening and closing")
    );
    assert_eq!(parse(&wire).unwrap(), sample);
}
