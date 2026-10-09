// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

fn capture(chunks: &[&[u8]]) -> (Vec<(String, bool)>, String) {
    let mut capture = Capture::default();
    let mut rows = Vec::new();
    for bytes in chunks {
        capture
            .process(bytes, |text, continued| {
                rows.push((text.into(), continued));
                Ok(())
            })
            .unwrap();
    }
    let tail = capture.tail().to_string();
    capture
        .finish(|text, continued| {
            rows.push((text.into(), continued));
            Ok(())
        })
        .unwrap();
    (rows, tail)
}

#[test]
fn plain_output_retains_tabs_spaces_blank_lines_and_legacy_carriage_returns() {
    let (rows, tail) = capture(&[b"\ttrail  \r", b"\n\rnext\r", b"end\n\npartial  "]);
    assert_eq!(
        rows,
        vec![
            ("\ttrail  ".into(), false),
            ("".into(), false),
            ("next".into(), false),
            ("end".into(), false),
            ("".into(), false),
            ("partial  ".into(), false)
        ]
    );
    assert_eq!(tail, "partial  ");
}

#[test]
fn all_chunk_boundaries_preserve_split_utf8_and_escape_sequences() {
    let input =
        "plain\t\x1b[31m東京 é\x1b[0m  \x1b]title\x07\r\nnext\x1b]hidden\x1b\\!\n".as_bytes();
    let expected = capture(&[input]).0;
    assert_eq!(
        expected,
        vec![("plain\t東京 é  ".into(), false), ("next!".into(), false)]
    );
    for split in 0..=input.len() {
        assert_eq!(capture(&[&input[..split], &input[split..]]).0, expected);
    }
    let bytes: Vec<_> = input.chunks(1).collect();
    assert_eq!(capture(&bytes).0, expected);
}

#[test]
fn invalid_utf8_is_lossy_and_incomplete_final_sequence_follows_final_cr() {
    let input = [b'a', 0xff, 0xe2, 0x82, b'x', 0xf0, 0x9f, 0x92, 0xa9];
    let expected = String::from_utf8_lossy(&input).into_owned();
    for split in 0..=input.len() {
        assert_eq!(
            capture(&[&input[..split], &input[split..]]).0,
            vec![(expected.clone(), false)]
        );
    }
    assert_eq!(
        capture(&[b"first\xe2\x82\r"]).0,
        vec![("first".into(), false), ("\u{fffd}".into(), false)]
    );
}

#[test]
fn long_unicode_rows_emit_bounded_continuations_without_losing_characters() {
    let text = format!("{}\tlast  ", "東京🙂".repeat(20_000));
    let (rows, _) = capture(&[text.as_bytes()]);
    assert!(rows.len() > 1);
    assert!(rows.iter().all(|(text, _)| text.len() <= MAX_PARTIAL_BYTES));
    assert!(
        rows[..rows.len() - 1]
            .iter()
            .all(|(_, continued)| *continued)
    );
    assert!(!rows.last().unwrap().1);
    assert_eq!(
        rows.iter()
            .map(|(text, _)| text.as_str())
            .collect::<String>(),
        text
    );
}

#[test]
fn huge_unterminated_escape_payloads_remain_constant_space_and_can_recovers_boundary() {
    let mut capture = Capture::default();
    let mut rows = Vec::new();
    capture
        .process(b"before\x1b]", |_, _| panic!("no line yet"))
        .unwrap();
    for _ in 0..100 {
        capture
            .process(&[b'x'; 16 * 1024], |_, _| panic!())
            .unwrap();
    }
    assert_eq!(capture.tail(), "before");
    assert_eq!(capture.partial.capacity(), MAX_PARTIAL_BYTES);
    capture
        .process(
            b"\x18\x1b]777;potyi-end;hidden\x07after\n",
            |text, continued| {
                rows.push((text.to_string(), continued));
                Ok(())
            },
        )
        .unwrap();
    assert_eq!(rows, [("beforeafter".into(), false)]);
    capture.reset();
    assert!(capture.tail().is_empty());
    assert_eq!(capture.utf8_len, 0);
}

#[test]
fn callback_failure_does_not_clear_the_unwritten_partial_row() {
    let mut capture = Capture::default();
    assert!(
        capture
            .process(b"keep\n", |_, _| Err(io::Error::other("disk unavailable")))
            .is_err()
    );
    assert_eq!(capture.tail(), "keep");
}

// Frozen pre-optimization byte loop. It deliberately shares only the unchanged
// UTF-8 decoder and row emitter, so the bulk path is checked against the exact
// previous control, CR, escape and continuation decisions.
fn process_reference(
    capture: &mut Capture,
    bytes: &[u8],
    mut emit: impl FnMut(&str, bool) -> io::Result<()>,
) -> io::Result<()> {
    for byte in bytes.iter().copied() {
        if byte == 0x18 {
            capture.state = State::Ground;
            continue;
        }
        match capture.state {
            State::Ground => match byte {
                0x1b => capture.state = State::Escape,
                b'\r' => capture.carriage_return = true,
                _ => {
                    if capture.carriage_return {
                        if byte != b'\n' {
                            capture.byte(b'\n', &mut emit)?;
                        }
                        capture.carriage_return = false;
                    }
                    if matches!(byte, b'\n' | b'\t') || byte >= 0x20 {
                        capture.byte(byte, &mut emit)?;
                    }
                }
            },
            State::Escape => {
                capture.state = match byte {
                    b'[' => State::Csi,
                    b']' => State::Osc,
                    _ => State::Ground,
                };
            }
            State::Csi => {
                if (0x40..=0x7e).contains(&byte) {
                    capture.state = State::Ground;
                }
            }
            State::Osc => {
                if byte == 7 {
                    capture.state = State::Ground;
                } else if byte == 0x1b {
                    capture.state = State::OscEscape;
                }
            }
            State::OscEscape => {
                capture.state = if byte == b'\\' {
                    State::Ground
                } else {
                    State::Osc
                };
            }
        }
    }
    Ok(())
}

fn same_capture_state(actual: &Capture, expected: &Capture) {
    assert_eq!(actual.tail(), expected.tail());
    assert_eq!(actual.carriage_return, expected.carriage_return);
    assert_eq!(actual.utf8_len, expected.utf8_len);
    assert_eq!(
        &actual.utf8[..actual.utf8_len],
        &expected.utf8[..expected.utf8_len]
    );
    assert_eq!(
        std::mem::discriminant(&actual.state),
        std::mem::discriminant(&expected.state)
    );
    assert_eq!(actual.partial.capacity(), MAX_PARTIAL_BYTES);
    assert!(actual.tail().len() <= MAX_PARTIAL_BYTES);
}

fn compare_reference(chunks: &[&[u8]]) -> Vec<(String, bool)> {
    let mut actual = Capture::default();
    let mut expected = Capture::default();
    let mut rows = Vec::new();
    let mut reference_rows = Vec::new();
    for bytes in chunks {
        actual
            .process(bytes, |text, continued| {
                rows.push((text.into(), continued));
                Ok(())
            })
            .unwrap();
        process_reference(&mut expected, bytes, |text, continued| {
            reference_rows.push((text.into(), continued));
            Ok(())
        })
        .unwrap();
        assert_eq!(rows, reference_rows);
        same_capture_state(&actual, &expected);
    }
    actual
        .finish(|text, continued| {
            rows.push((text.into(), continued));
            Ok(())
        })
        .unwrap();
    expected
        .finish(|text, continued| {
            reference_rows.push((text.into(), continued));
            Ok(())
        })
        .unwrap();
    assert_eq!(rows, reference_rows);
    same_capture_state(&actual, &expected);
    rows
}

#[test]
fn ascii_runs_match_byte_reference_at_every_control_and_utf8_split() {
    let cases: &[&[u8]] = &[
        b"ascii\ttrail  \r\n\rnext\rend\n\npartial  ",
        "text\t\x1b[31m東京 e\u{301}🙂\x1b[0m \x1b]hidden\x1b\\!\n".as_bytes(),
        b"a\xff\xe2\x82x\xf0\x9f\x92\xa9\xc0\xaf\xed\xa0\x80\xf4\x90\x80\x80\xe2\x82\r",
        b"before\xe2\x1b[31m\x82\x18\xac\r\x1b]hide\x07tail\n",
        b"before\x1b]unfinished\x18\x1b]777;potyi-end;hidden\x07after\n",
        b"a\x00b\x01c\x7fd\x1b[2\x18e\r\x1b]hidden\x1b\\\tf\n",
    ];
    for bytes in cases {
        for split in 0..=bytes.len() {
            compare_reference(&[&bytes[..split], &bytes[split..]]);
        }
        for chunk_size in [1, 2, 3, 7, 1_024] {
            let chunks: Vec<_> = bytes.chunks(chunk_size).collect();
            compare_reference(&chunks);
        }
    }
}

#[test]
fn ascii_runs_keep_exact_partial_cap_and_mixed_unicode_continuations() {
    let prefix = "a".repeat(MAX_PARTIAL_BYTES);
    let input = format!("{prefix}\t東京🙂\n");
    let expected = vec![(prefix, true), ("\t東京🙂".into(), false)];
    assert_eq!(compare_reference(&[input.as_bytes()]), expected);
    let mixed = format!(
        "{}é{}🙂\r\n{}\tfinal  ",
        "x".repeat(MAX_PARTIAL_BYTES - 1),
        "y".repeat(MAX_PARTIAL_BYTES - 3),
        "東京🙂".repeat(20_000)
    );
    for chunk_size in [31, 1_024, 16_384, MAX_PARTIAL_BYTES] {
        let chunks: Vec<_> = mixed.as_bytes().chunks(chunk_size).collect();
        compare_reference(&chunks);
    }
}

#[test]
fn ascii_runs_match_reference_across_repeated_controls_and_malformed_bytes() {
    let tokens: &[&[u8]] = &[
        b"plain ascii words\t ",
        "東京🙂é".as_bytes(),
        b"\n",
        b"\r",
        b"\r\n",
        b"\x1b[31m",
        b"\x1b[0m",
        b"\x1b]secret\x07",
        b"\x1b]secret\x1b\\",
        b"\x18",
        b"\0\x01\x7f",
        b"\xff\xe2",
        b"\x82\xac",
        b"\xf0\x9f",
        b"\x92\xa9",
        b"\xed\xa0\x80",
        b"\x1b]unfinished",
        b"\x1b[unfinished",
    ];
    let mut bytes = Vec::new();
    let mut random = 0x31f0_573du32;
    for _ in 0..2_048 {
        random = random.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        bytes.extend_from_slice(tokens[random as usize % tokens.len()]);
    }
    for chunk_size in [1, 2, 7, 31, 1_024] {
        let chunks: Vec<_> = bytes.chunks(chunk_size).collect();
        compare_reference(&chunks);
    }
}

#[test]
fn ascii_runs_preserve_unwritten_row_on_callback_errors_and_finish_errors() {
    let input = format!("{}xé\nnext\rfinal", "x".repeat(MAX_PARTIAL_BYTES));
    // Append the incomplete sequence as raw bytes to exercise finish's
    // replacement after the final CR.
    let mut bytes = input.into_bytes();
    bytes.extend_from_slice(b"\xe2\x82\r");
    for fail_at in 0..6 {
        let mut actual = Capture::default();
        let mut expected = Capture::default();
        let mut actual_calls = 0;
        let mut expected_calls = 0;
        let mut actual_rows = Vec::new();
        let mut expected_rows = Vec::new();
        let mut emit_actual = |text: &str, continued| {
            let call = actual_calls;
            actual_calls += 1;
            if call == fail_at {
                return Err(io::Error::other("disk unavailable"));
            }
            actual_rows.push((text.to_string(), continued));
            Ok(())
        };
        let mut emit_expected = |text: &str, continued| {
            let call = expected_calls;
            expected_calls += 1;
            if call == fail_at {
                return Err(io::Error::other("disk unavailable"));
            }
            expected_rows.push((text.to_string(), continued));
            Ok(())
        };
        let actual_result = actual.process(&bytes, &mut emit_actual);
        let expected_result = process_reference(&mut expected, &bytes, &mut emit_expected);
        assert_eq!(actual_result.is_ok(), expected_result.is_ok());
        same_capture_state(&actual, &expected);
        if actual_result.is_ok() {
            assert_eq!(
                actual.finish(&mut emit_actual).is_ok(),
                expected.finish(&mut emit_expected).is_ok()
            );
            same_capture_state(&actual, &expected);
        }
        assert_eq!(actual_calls, expected_calls);
        assert_eq!(actual_rows, expected_rows);
    }
}
