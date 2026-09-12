// Pötyi - Lightweight text editor
// SPDX-License-Identifier: GPL-3.0-or-later
//! Explicit quick fixes/refactors; resolve edits before the existing review/apply flow.
use super::*;
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub(crate) struct CodeActionItem {
    pub title: String,
    pub disabled: Option<String>,
    pub value: Value,
}

#[derive(Debug)]
pub(crate) struct CodeActions {
    pub items: Vec<CodeActionItem>,
    pub started: SystemTime,
}

fn actions(result: Value, started: SystemTime) -> Result<CodeActions, String> {
    if result.is_null() {
        return Ok(CodeActions {
            items: Vec::new(),
            started,
        });
    }
    let raw = result.as_array().ok_or("Invalid code-action response")?;
    if raw.len() > 256 || result.to_string().len() > 2 * 1024 * 1024 {
        return Err("Language server returned too many code actions".into());
    }
    let mut items = Vec::new();
    for value in raw {
        let title = value["title"]
            .as_str()
            .ok_or("Code action has no title")?
            .chars()
            .filter(|c| !c.is_control())
            .take(240)
            .collect::<String>();
        let disabled = value["disabled"]["reason"].as_str().map(str::to_owned)
            .or_else(|| value.get("command").filter(|v| !v.is_null()).map(|_|
                "This action requires a server command; only previewable workspace edits are supported".into()));
        items.push(CodeActionItem {
            title,
            disabled,
            value: value.clone(),
        });
    }
    items.sort_by_key(|a| a.value["isPreferred"] != true);
    Ok(CodeActions { items, started })
}

fn coordinate(value: &Value) -> Option<(u64, u64)> {
    Some((value["line"].as_u64()?, value["character"].as_u64()?))
}

impl Session {
    pub(super) fn execute_code_action(&mut self, request: &Request) -> Result<Reply, String> {
        if !self.code_actions {
            return Err("This language server does not provide code actions".into());
        }
        let focused = request.documents.first().ok_or("No document supplied")?;
        let uri = file_uri(&focused.path)?;
        if self.reports_readiness {
            self.transport.wait_until_ready(TIMEOUT)?;
        }
        match &request.action {
            Action::CodeActions {
                anchor,
                refactor_only,
            } => {
                let started = SystemTime::now();
                let start = position(&focused.text, request.cursor.min(*anchor))?;
                let end = position(&focused.text, request.cursor.max(*anchor))?;
                let version = self.documents[&focused.path].1;
                let diagnostics = if *refactor_only {
                    Vec::new()
                } else if self.pull_diagnostics {
                    let report = self.transport.request(
                        "textDocument/diagnostic",
                        json!({"textDocument":{"uri":uri}}),
                        TIMEOUT,
                    )?;
                    report["items"].as_array().cloned().unwrap_or_default()
                } else {
                    self.transport.action_diagnostics(&uri, version)?
                };
                let diagnostics: Vec<_> = diagnostics
                    .into_iter()
                    .take(256)
                    .filter(|d| {
                        match (
                            coordinate(&d["range"]["start"]),
                            coordinate(&d["range"]["end"]),
                        ) {
                            (Some(a), Some(b)) => {
                                a <= coordinate(&end).unwrap() && b >= coordinate(&start).unwrap()
                            }
                            _ => false,
                        }
                    })
                    .collect();
                let mut context = json!({"diagnostics":diagnostics,"triggerKind":1});
                if *refactor_only {
                    context["only"] = json!(["refactor"]);
                }
                let result = self.transport.request("textDocument/codeAction", json!({
                    "textDocument":{"uri":uri}, "range":{"start":start,"end":end},"context":context
                }), TIMEOUT)?;
                Ok(Reply::CodeActions(actions(result, started)?))
            }
            Action::ResolveCodeAction { action, started } => {
                let mut action = action.clone();
                if action.get("edit").is_none_or(Value::is_null) && self.resolve_actions {
                    action = self
                        .transport
                        .request("codeAction/resolve", action, TIMEOUT)?;
                }
                if let Some(reason) = action["disabled"]["reason"].as_str() {
                    return Err(reason.into());
                }
                if action.get("command").is_some_and(|v| !v.is_null()) {
                    return Err("This action requires a server command; it cannot be previewed as a workspace edit".into());
                }
                let edit = action
                    .get("edit")
                    .ok_or("This action returned no workspace edit")?;
                crate::workspace_edit::prepare(
                    edit,
                    &self.root,
                    &request.documents,
                    &self.documents,
                    *started,
                )
                .map(Reply::Rename)
            }
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn command_actions_are_unavailable_and_preferred_edits_come_first() {
        let list = actions(
            json!([
                {"title":"Server operation","command":"server.run"},
                {"title":"Quick fix","isPreferred":true,"edit":{}},
                {"title":"Disabled","disabled":{"reason":"Not here"}}
            ]),
            SystemTime::now(),
        )
        .unwrap();
        assert_eq!(list.items[0].title, "Quick fix");
        assert!(
            list.items[1]
                .disabled
                .as_ref()
                .unwrap()
                .contains("server command")
        );
        assert_eq!(list.items[2].disabled.as_deref(), Some("Not here"));
        assert!(actions(json!([{}]), SystemTime::now()).is_err());
        assert!(actions(json!(vec![json!({"title":"fix"}); 257]), SystemTime::now()).is_err());
    }
}
