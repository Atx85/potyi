// Pötyi - Lightweight text editor
// SPDX-License-Identifier: GPL-3.0-or-later

use super::{Action, Reply, Request, Session, TIMEOUT, file_uri, position};
use serde_json::{Value, json};

#[derive(Debug)]
pub(crate) struct Item {
    pub label: String,
    pub value: Value,
}

#[derive(Debug)]
pub(crate) struct Edit {
    pub text: String,
    pub cursor: usize,
}

impl Session {
    pub(super) fn execute_completion(&mut self, request: &Request) -> Result<Reply, String> {
        let doc = request.documents.first().ok_or("No document supplied")?;
        match &request.action {
            Action::Complete { trigger, cancel } => {
                if !self.completion_triggers.contains(trigger) {
                    return Ok(Reply::Completions(vec![]));
                }
                if self.reports_readiness {
                    self.transport
                        .wait_until_ready_cancellable(TIMEOUT, Some(cancel))?;
                }
                let result = self.transport.request_cancellable("textDocument/completion", json!({
                    "textDocument":{"uri":file_uri(&doc.path)?}, "position":position(&doc.text, request.cursor)?,
                    "context":{"triggerKind":2,"triggerCharacter":trigger}
                }), TIMEOUT, Some(cancel))?;
                Ok(Reply::Completions(items(&result)?))
            }
            Action::ResolveCompletion { item, cancel } => {
                let item = if self.resolve_completion {
                    let resolved = self.transport.request_cancellable(
                        "completionItem/resolve",
                        item.clone(),
                        TIMEOUT,
                        Some(cancel),
                    )?;
                    // Retain original fields if a server only supplies resolved fields.
                    let mut merged = item.clone();
                    let object = merged.as_object_mut().ok_or("Invalid completion item")?;
                    for (key, value) in resolved.as_object().ok_or("Invalid resolved completion")? {
                        object.insert(key.clone(), value.clone());
                    }
                    merged
                } else {
                    item.clone()
                };
                prepare(&doc.text, request.cursor, &item).map(Reply::CompletionEdit)
            }
            _ => unreachable!(),
        }
    }
}

fn items(result: &Value) -> Result<Vec<Item>, String> {
    if result.is_null() {
        return Ok(vec![]);
    }
    // itemDefaults are deliberately not advertised: every item must be complete.
    if result
        .get("itemDefaults")
        .is_some_and(|v| v.as_object().is_some_and(|o| !o.is_empty()))
    {
        return Err("Server returned unsupported completion defaults".into());
    }
    let values = result
        .as_array()
        .or_else(|| result["items"].as_array())
        .ok_or("Invalid completion list")?;
    let mut sorted: Vec<_> = values.iter().collect();
    sorted.sort_by_key(|v| {
        v["sortText"]
            .as_str()
            .or_else(|| v["label"].as_str())
            .unwrap_or("")
    });
    let mut output = Vec::new();
    let mut bytes = 0;
    for value in sorted {
        if value["insertTextFormat"] == 2 {
            continue;
        }
        let Some(label) = value["label"].as_str() else {
            continue;
        };
        bytes += value.to_string().len();
        if bytes > 512 * 1024 || output.len() == 256 {
            break;
        }
        let label = label
            .chars()
            .filter(|c| !c.is_control())
            .take(120)
            .collect();
        output.push(Item {
            label,
            value: value.clone(),
        });
    }
    Ok(output)
}

/// Prepare the entire change off the UI thread. Primary insertion and optional
/// imports are validated against one UTF-16 snapshot and committed as one undo.
fn prepare(text: &str, cursor: usize, item: &Value) -> Result<Edit, String> {
    if item["insertTextFormat"] == 2 {
        return Err("Snippet completions are not supported yet".into());
    }
    if item.get("command").is_some_and(|v| !v.is_null()) {
        return Err(
            "This completion requires a server command; choose a plain member completion".into(),
        );
    }
    let range = |value: &Value| -> Result<(usize, usize), String> {
        let start = crate::workspace_edit::byte_position(text, &value["start"])?;
        let end = crate::workspace_edit::byte_position(text, &value["end"])?;
        if start > end {
            return Err("Invalid completion edit range".into());
        }
        Ok((start, end))
    };
    let (start, end, new_text) = if let Some(edit) = item.get("textEdit").filter(|v| !v.is_null()) {
        let (start, end) = range(
            edit.get("range")
                .or_else(|| edit.get("insert"))
                .ok_or("Missing completion range")?,
        )?;
        if start > cursor || end < cursor {
            return Err("Completion edit does not contain the cursor".into());
        }
        (
            start,
            end,
            edit["newText"].as_str().ok_or("Missing completion text")?,
        )
    } else {
        (
            cursor,
            cursor,
            item["insertText"]
                .as_str()
                .or_else(|| item["label"].as_str())
                .ok_or("Missing completion text")?,
        )
    };
    let mut edits = vec![(start, end, new_text)];
    if let Some(additional) = item.get("additionalTextEdits").filter(|v| !v.is_null()) {
        let additional = additional
            .as_array()
            .ok_or("Invalid additional completion edits")?;
        if additional.len() > 128 {
            return Err("Too many completion edits".into());
        }
        for edit in additional {
            let (start, end) = range(&edit["range"])?;
            edits.push((
                start,
                end,
                edit["newText"].as_str().ok_or("Missing completion text")?,
            ));
        }
    }
    edits.sort_by_key(|(start, end, _)| (*start, *end));
    for pair in edits.windows(2) {
        if pair[0].1 > pair[1].0 || pair[0].0 == pair[1].0 {
            return Err("Overlapping completion edits".into());
        }
    }
    let mut output = String::new();
    let mut offset = 0;
    let mut final_cursor = None;
    for (from, to, replacement) in edits {
        if output.len() + from - offset + replacement.len() > super::MAX_DOCUMENT_BYTES {
            return Err("Completion exceeds the 2 MiB limit".into());
        }
        output.push_str(&text[offset..from]);
        output.push_str(replacement);
        if from == start {
            final_cursor = Some(output.len());
        }
        offset = to;
    }
    if output.len() + text.len() - offset > super::MAX_DOCUMENT_BYTES {
        return Err("Completion exceeds the 2 MiB limit".into());
    }
    output.push_str(&text[offset..]);
    Ok(Edit {
        text: output,
        cursor: final_cursor.ok_or("Missing primary completion edit")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pos(line: usize, character: usize) -> Value {
        json!({"line":line,"character":character})
    }
    #[test]
    fn unicode_completion_and_import_are_one_prepared_change() {
        let text = "// 😀\nobj.me";
        let item = json!({"label":"member","textEdit":{"range":{"start":pos(1,4),"end":pos(1,6)},"newText":"member"},"additionalTextEdits":[{"range":{"start":pos(0,0),"end":pos(0,0)},"newText":"use Foo;\n"}]});
        let result = prepare(text, text.len(), &item).unwrap();
        assert_eq!(result.text, "use Foo;\n// 😀\nobj.member");
        assert_eq!(result.cursor, result.text.len());
        let mut invalid = item.clone();
        invalid["additionalTextEdits"][0]["range"]["start"] = pos(1, 5);
        invalid["additionalTextEdits"][0]["range"]["end"] = pos(1, 5);
        assert!(
            prepare(text, text.len(), &invalid)
                .unwrap_err()
                .contains("Overlapping")
        );
    }
    #[test]
    fn insertion_range_preserves_suffix_and_unicode_boundaries() {
        let item = json!({"label":"Foo","textEdit":{"insert":{"start":pos(0,4),"end":pos(0,4)},"replace":{"start":pos(0,4),"end":pos(0,8)},"newText":"Foo"}});
        assert_eq!(prepare("obj.tail", 4, &item).unwrap().text, "obj.Footail");
        let invalid = json!({"label":"x","textEdit":{"range":{"start":pos(0,1),"end":pos(0,2)},"newText":"x"}});
        assert!(prepare("😀", 4, &invalid).is_err());
        assert!(
            prepare(
                "obj.",
                4,
                &json!({"label":"method($0)","insertTextFormat":2})
            )
            .is_err()
        );
    }
    #[test]
    fn lists_are_sorted_bounded_and_exclude_snippets() {
        let input = json!({"isIncomplete":true,"items":[{"label":"z","sortText":"0"},{"label":"a"},{"label":"snippet","insertTextFormat":2}]});
        let result = items(&input).unwrap();
        assert_eq!(
            result.iter().map(|i| i.label.as_str()).collect::<Vec<_>>(),
            ["z", "a"]
        );
        assert_eq!(
            items(&json!(vec![json!({"label":"x"}); 1000]))
                .unwrap()
                .len(),
            256
        );
    }
}
