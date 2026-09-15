"""Deterministic stdio language server for Pötyi's integration tests."""
import json
import sys
import time

mode = sys.argv[1]
documents = {}
changes = []
versions = {}
hover_attempts = 0
pending_hover = None


def send(value):
    body = json.dumps({"jsonrpc": "2.0", **value}, ensure_ascii=False).encode()
    sys.stdout.buffer.write(f"Content-Length: {len(body)}\r\n\r\n".encode())
    sys.stdout.buffer.write(body)
    sys.stdout.buffer.flush()


def offset(text, position):
    lines = text.splitlines(keepends=True)
    line = position["line"]
    prefix = "".join(lines[:line])
    tail = lines[line] if line < len(lines) else ""
    units = position["character"]
    count = 0
    for ch in tail:
        if units == 0:
            break
        units -= len(ch.encode("utf-16-le")) // 2
        count += 1
    assert units == 0
    return len(prefix) + count


while True:
    length = None
    while True:
        line = sys.stdin.buffer.readline()
        if not line:
            sys.exit(0)
        if line == b"\r\n":
            break
        key, value = line.decode().split(":", 1)
        if key.lower() == "content-length":
            length = int(value)
    message = json.loads(sys.stdin.buffer.read(length))
    method = message.get("method")
    params = message.get("params") or {}
    if method == "initialize":
        if mode == "hang":
            time.sleep(60)
        if mode == "crash":
            sys.exit(1)
        assert params["capabilities"]["general"]["positionEncodings"] == ["utf-16"]
        send({"id": message["id"], "result": {"capabilities": {
            "hoverProvider": True, "definitionProvider": True,
            "codeActionProvider": {"resolveProvider": True},
            "completionProvider": {"triggerCharacters": [".", ":", ">"], "resolveProvider": True},
            "textDocumentSync": {"openClose": True, "change": 1 if mode == "full" else 2}
        }}})
    elif method == "textDocument/didOpen":
        doc = params["textDocument"]
        documents[doc["uri"]] = doc["text"]
        versions[doc["uri"]] = doc["version"]
        if mode == "actions":
            send({"method": "textDocument/publishDiagnostics", "params": {
                "uri": doc["uri"], "version": doc["version"], "diagnostics": [{
                    "range": {"start": {"line": 0, "character": 2}, "end": {"line": 0, "character": 9}},
                    "message": "Missing import", "data": {"fix": "import"}
                }]}})
    elif method == "textDocument/didChange":
        uri = params["textDocument"]["uri"]
        assert params["textDocument"]["version"] > versions[uri]
        versions[uri] = params["textDocument"]["version"]
        for change in params["contentChanges"]:
            changes.append(change)
            text = documents[uri]
            if "range" in change:
                start = offset(text, change["range"]["start"])
                end = offset(text, change["range"]["end"])
                documents[uri] = text[:start] + change["text"] + text[end:]
            else:
                documents[uri] = change["text"]
    elif method == "textDocument/didClose":
        documents.pop(params["textDocument"]["uri"], None)
    elif method == "textDocument/codeAction":
        assert params["range"] == {"start": {"line": 0, "character": 2}, "end": {"line": 0, "character": 9}}
        assert params["context"]["diagnostics"][0]["data"]["fix"] == "import"
        send({"id": message["id"], "result": [
            {"title": "Disabled fix", "disabled": {"reason": "Not applicable"}},
            {"title": "Add missing import", "kind": "quickfix", "isPreferred": True,
             "data": {"uri": params["textDocument"]["uri"]}}
        ]})
    elif method == "codeAction/resolve":
        params["edit"] = {"changes": {params["data"]["uri"]: [{
            "range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 0}},
            "newText": "use std::fmt;\n"
        }]}}
        send({"id": message["id"], "result": params})
    elif method == "textDocument/completion":
        assert params["context"]["triggerKind"] == 2
        assert params["context"]["triggerCharacter"] in [".", ":", ">"]
        if mode == "slow-completion":
            time.sleep(0.25)
        send({"id": message["id"], "result": {"isIncomplete": False, "items": [
            {"label": "position", "sortText": "1", "data": {"member": "position"}},
            {"label": "Translate", "sortText": "2", "data": {"member": "Translate"}}
        ]}})
    elif method == "completionItem/resolve":
        params["insertText"] = params["data"]["member"]
        send({"id": message["id"], "result": params})
    elif method == "textDocument/hover":
        hover_attempts += 1
        if mode == "crash-hover":
            sys.exit(1)
        if mode == "timeout-hover" and hover_attempts == 1:
            pending_hover = message["id"]
            continue
        if mode == "feature-error" and hover_attempts == 1:
            send({"id": message["id"], "error": {"code": -32602, "message": "No references found at position"}})
            continue
        send({"id": 987, "method": "workspace/configuration", "params": {"items": [{}]}})
        text = json.dumps({"documents": documents, "changes": changes, "position": params["position"]}, ensure_ascii=False)
        send({"id": message["id"], "result": {"contents": {"kind": "plaintext", "value": text}}})
    elif method == "textDocument/definition":
        send({"id": message["id"], "result": [{"targetUri": params["textDocument"]["uri"],
            "targetSelectionRange": {"start": {"line": 0, "character": 2}, "end": {"line": 0, "character": 3}}}]})
    elif method == "$/cancelRequest" and pending_hover == params["id"]:
        # Send a late response to exercise request-ID isolation after a timeout.
        send({"id": pending_hover, "result": {"contents": "stale hover"}})
        pending_hover = None
    elif method == "shutdown":
        send({"id": message["id"], "result": None})
    elif method == "exit":
        sys.exit(0)
