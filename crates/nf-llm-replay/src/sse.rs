//! Server-sent events: split a `text/event-stream` body into raw events.

/// The raw events of `body`, each with its terminating blank line (`\n`, `\r\n` or `\r` line
/// endings, mixed freely); an unterminated tail is kept as the last element. Concatenating the
/// result gives `body` back byte for byte.
pub(crate) fn split_events(body: &str) -> Vec<String> {
    let bytes = body.as_bytes();
    let mut events = Vec::new();
    let (mut start, mut line, mut i) = (0, 0, 0);
    while let Some(&b) = bytes.get(i) {
        let eol = match b {
            b'\r' if bytes.get(i + 1) == Some(&b'\n') => 2,
            b'\r' | b'\n' => 1,
            _ => {
                i += 1;
                continue;
            }
        };
        let blank = i == line;
        i += eol;
        line = i;
        if blank {
            // Cut points sit right after ASCII line endings, so they are char boundaries.
            events.extend(body.get(start..i).map(str::to_owned));
            start = i;
        }
    }
    events.extend(
        body.get(start..)
            .filter(|tail| !tail.is_empty())
            .map(str::to_owned),
    );
    events
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIXED: &str =
        "event: a\ndata: 1\n\nevent: b\r\ndata: 2\r\n\r\ndata: 3\r\rdata: [DONE]\n\n";

    #[test]
    fn splits_lf_crlf_and_cr_terminated_events() {
        assert_eq!(
            split_events(MIXED),
            [
                "event: a\ndata: 1\n\n",
                "event: b\r\ndata: 2\r\n\r\n",
                "data: 3\r\r",
                "data: [DONE]\n\n",
            ]
        );
        assert_eq!(split_events(""), Vec::<String>::new());
    }

    #[test]
    fn concatenated_events_equal_the_original_bytes() {
        for body in [
            MIXED,
            "\n\ndata: x\n\n",
            "data: é\r\n\r\n: comment\n\n",
            "data: a\r\n\n",
        ] {
            assert_eq!(split_events(body).concat(), body, "{body:?}");
        }
    }

    #[test]
    fn unterminated_trailing_event_is_kept() {
        assert_eq!(
            split_events("data: 1\n\ndata: partial\n"),
            ["data: 1\n\n", "data: partial\n"]
        );
        assert_eq!(split_events("data: x"), ["data: x"]);
    }
}
