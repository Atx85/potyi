import { copyDefinition, exportToml, fileName, makeEngine, MAX_DEFINITION_BYTES } from "./model.mjs";

const $ = id => document.getElementById(id);
const encoder = new TextEncoder();
const decoder = new TextDecoder();
let engine, presets = [], definition, selected = 0, filename = "syntax_python.toml";
let pending, validToml = "", revision = 0;

function message(text, error = false) {
  $("validation").textContent = text;
  $("validation").classList.toggle("error", error);
}
function invalidate() {
  validToml = "";
  $("download").disabled = true;
  $("copy-toml").disabled = true;
}
function showRule() {
  const rule = definition.rules[selected];
  $("rule-name").value = rule.name;
  $("rule-pattern").value = rule.pattern;
  $("rule-color").value = rule.color;
  $("editing-label").textContent = `Rule ${selected + 1} of ${definition.rules.length}`;
  $("move-up").disabled = selected === 0;
  $("move-down").disabled = selected === definition.rules.length - 1;
  $("remove-rule").disabled = definition.rules.length === 1;
  $("add-rule").disabled = definition.rules.length >= 32;
  $("rule-count").textContent = definition.rules.length;
}
function renderRules() {
  const fragment = document.createDocumentFragment();
  definition.rules.forEach((rule, index) => {
    const row = document.createElement("div");
    row.className = `rule-row${index === selected ? " selected" : ""}`;
    const color = document.createElement("input");
    color.type = "color";
    color.value = /^#[a-f0-9]{6}$/i.test(rule.color) ? rule.color : "#dcdcdc";
    color.setAttribute("aria-label", `Color for ${rule.name || "unnamed"} rule ${index + 1}`);
    color.addEventListener("input", () => {
      rule.color = color.value;
      colorText.textContent = color.value.toUpperCase();
      if (selected === index) $("rule-color").value = color.value;
      changed();
    });
    const button = document.createElement("button");
    button.type = "button";
    button.className = "rule-select";
    button.setAttribute("aria-pressed", String(selected === index));
    button.setAttribute("aria-label", `Edit ${rule.name || "unnamed"} rule ${index + 1}`);
    const name = document.createElement("strong"); name.textContent = rule.name || "Unnamed rule";
    const colorText = document.createElement("small"); colorText.textContent = rule.color.toUpperCase();
    button.append(name, colorText);
    button.addEventListener("click", () => { selected = index; renderRules(); showRule(); $("rule-name").focus(); });
    row.append(color, button); fragment.append(row);
  });
  const scroll = $("rule-list").scrollTop;
  $("rule-list").replaceChildren(fragment);
  $("rule-list").scrollTop = scroll;
}
function paint(source, lines) {
  const fragment = document.createDocumentFragment();
  const sourceLines = source.split("\n").slice(0, 200);
  sourceLines.forEach((text, index) => {
    const line = document.createElement("span"); line.className = "code-line";
    const number = document.createElement("span"); number.className = "line-number";
    number.setAttribute("aria-hidden", "true"); number.textContent = index + 1;
    line.append(number);
    const bytes = encoder.encode(text);
    let offset = 0;
    for (const [start, end, color] of lines?.[index] || []) {
      line.append(document.createTextNode(decoder.decode(bytes.subarray(offset, start))));
      const span = document.createElement("span"); span.style.color = color;
      span.textContent = decoder.decode(bytes.subarray(start, end));
      line.append(span); offset = end;
    }
    line.append(document.createTextNode(decoder.decode(bytes.subarray(offset))));
    fragment.append(line);
  });
  $("preview-code").replaceChildren(fragment);
  $("preview-stats").textContent = `${sourceLines.length} line${sourceLines.length === 1 ? "" : "s"} · ${encoder.encode(source).length.toLocaleString()} bytes${source.split("\n").length > 200 ? " · first 200 lines shown" : ""}`;
}
function preview() {
  const source = $("sample-text").value;
  try {
    const result = engine.highlight(source);
    paint(source, result.lines);
    message(`Definition checked. ${definition.rules.length} rules ready for Pötyi.`);
  } catch (error) {
    paint(source, null);
    message(`Preview: ${error.message} Your checked definition can still be downloaded.`, true);
  }
}
function validate() {
  clearTimeout(pending);
  if (!engine || !definition) return;
  invalidate();
  const source = exportToml(definition);
  $("export-code").textContent = source;
  $("download-name").textContent = filename;
  $("sample-filename").textContent = `example.${definition.extensions[0] || "txt"}`;
  try {
    engine.configure(source);
    validToml = source;
    $("download").disabled = false;
    $("copy-toml").disabled = false;
    $("download-detail").textContent = `${definition.rules.length} rules · ${(encoder.encode(source).length / 1024).toFixed(1)} KB · UTF-8`;
    $("rule-pattern").removeAttribute("aria-invalid");
    preview();
  } catch (error) {
    message(error.message, true);
    $("download-detail").textContent = "Fix the definition to enable download.";
    paint($("sample-text").value, null);
  }
}
function changed() {
  revision++;
  invalidate();
  message("Checking your changes…");
  clearTimeout(pending);
  pending = setTimeout(validate, 180);
}
function applyDefinition(value, name, sample) {
  revision++;
  definition = copyDefinition(value); selected = 0; filename = name;
  $("language-name").value = definition.name;
  $("extensions").value = definition.extensions.join(", ");
  if (sample !== undefined) $("sample-text").value = sample;
  renderRules(); showRule(); validate();
}
function tab(view) {
  const isPreview = view === "preview";
  for (const name of ["preview", "sample"]) {
    const active = (name === "preview") === isPreview;
    $(`${name}-tab`).setAttribute("aria-selected", String(active));
    $(`${name}-tab`).tabIndex = active ? 0 : -1;
    $(`${name}-view`).hidden = !active;
  }
}
function downloadFile(text, name) {
  const url = URL.createObjectURL(new Blob([text], { type: "application/toml;charset=utf-8" }));
  const link = document.createElement("a"); link.href = url; link.download = name;
  document.body.append(link); link.click(); link.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
function connectControls() {
  $("preset").addEventListener("change", () => {
    const preset = presets[Number($("preset").value)];
    applyDefinition(preset.definition, preset.filename, preset.sample);
  });
  $("language-name").addEventListener("input", () => { definition.name = $("language-name").value; changed(); });
  $("extensions").addEventListener("input", () => { definition.extensions = $("extensions").value.split(",").map(s => s.trim()).filter(Boolean); changed(); });
  for (const [id, field] of [["rule-name", "name"], ["rule-pattern", "pattern"], ["rule-color", "color"]]) {
    $(id).addEventListener("input", () => { definition.rules[selected][field] = $(id).value; if (field !== "pattern") renderRules(); changed(); });
  }
  $("add-rule").addEventListener("click", () => {
    if (definition.rules.length >= 32) return;
    definition.rules.push({ name: "custom", pattern: "\\bexample\\b", color: "#DCDCAA" });
    selected = definition.rules.length - 1; renderRules(); showRule(); changed(); $("rule-name").focus();
  });
  $("remove-rule").addEventListener("click", () => {
    if (definition.rules.length <= 1) return;
    definition.rules.splice(selected, 1); selected = Math.min(selected, definition.rules.length - 1);
    renderRules(); showRule(); changed();
  });
  for (const [id, direction] of [["move-up", -1], ["move-down", 1]]) {
    $(id).addEventListener("click", () => {
      const next = selected + direction;
      if (next < 0 || next >= definition.rules.length) return;
      [definition.rules[selected], definition.rules[next]] = [definition.rules[next], definition.rules[selected]];
      selected = next; renderRules(); showRule(); changed();
    });
  }
  $("sample-text").addEventListener("input", () => {
    revision++; clearTimeout(pending);
    pending = setTimeout(() => validToml ? preview() : validate(), 180);
  });
  for (const name of ["preview", "sample"]) {
    $(`${name}-tab`).addEventListener("click", () => tab(name));
    $(`${name}-tab`).addEventListener("keydown", event => {
      if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
      event.preventDefault(); const next = event.key === "Home" ? "preview" : event.key === "End" ? "sample" : name === "preview" ? "sample" : "preview";
      tab(next); $(`${next}-tab`).focus();
    });
  }
  $("download").addEventListener("click", () => {
    validate();
    if (validToml) {
      downloadFile(validToml, filename);
      message(`Download started: ${filename}. Add it to config/syntax/ and reopen your document.`);
    }
  });
  $("copy-toml").addEventListener("click", async () => {
    validate(); if (!validToml) return;
    try { await navigator.clipboard.writeText(validToml); message("File contents copied. Save them as a .toml file in config/syntax/."); }
    catch { message("Clipboard access is unavailable. Use Download .toml instead.", true); }
  });
  $("import-file").addEventListener("change", async () => {
    const file = $("import-file").files[0]; $("import-file").value = "";
    if (!file) return;
    const started = revision;
    try {
      if (file.size > MAX_DEFINITION_BYTES) throw new Error("Keep the definition under 64 KiB.");
      const source = new TextDecoder("utf-8", { fatal: true }).decode(await file.arrayBuffer());
      if (started !== revision) { message("Import skipped because you edited the current definition. Select the file again to import it.", true); return; }
      const value = engine.configure(source);
      const name = /^[a-zA-Z0-9_.-]+\.toml$/i.test(file.name) ? file.name : fileName(value.name);
      $("preset").value = "";
      applyDefinition(value, name);
    } catch (error) {
      // An invalid import must not replace or disable the current design.
      validate(); message(`Could not import: ${error.message} Your current design is unchanged.`, true);
    }
  });
}
async function start() {
  try {
    const [wasmResponse, presetsResponse] = await Promise.all([fetch("engine.wasm"), fetch("presets.json")]);
    if (!wasmResponse.ok || !presetsResponse.ok) throw new Error("A designer file could not be loaded. Reload the page to try again.");
    const [{ instance }, values] = await Promise.all([WebAssembly.instantiate(await wasmResponse.arrayBuffer(), {}), presetsResponse.json()]);
    engine = makeEngine(instance.exports); presets = values;
    const blank = document.createElement("option"); blank.value = ""; blank.textContent = "Imported definition"; blank.disabled = true;
    $("preset").replaceChildren(blank);
    presets.forEach((preset, index) => { const option = document.createElement("option"); option.value = index; option.textContent = preset.definition.name; $("preset").append(option); });
    const index = presets.findIndex(preset => preset.definition.name === "Python");
    $("preset").value = index; $("preset").disabled = false;
    connectControls();
    applyDefinition(presets[index].definition, presets[index].filename, presets[index].sample);
    $("engine-status").hidden = true;
    $("workspace").setAttribute("aria-busy", "false");
  } catch (error) {
    invalidate();
    $("engine-status").textContent = `The designer could not start. ${error.message}`;
    $("engine-status").classList.add("error");
    $("workspace").setAttribute("aria-busy", "false");
  }
}
// No network calls, timers, or background tasks after initial loading unless
// the user edits, imports, copies, or downloads something.
start();
