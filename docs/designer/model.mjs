export const MAX_DEFINITION_BYTES = 64 * 1024;
export const MAX_SAMPLE_BYTES = 16 * 1024;

// JSON basic-string escaping is also valid TOML; DEL needs an explicit escape.
export function tomlString(value) {
  return JSON.stringify(String(value)).replace(/\u007f/g, "\\u007f");
}
export function exportToml(definition) {
  const lines = [
    "# Created with the Pötyi syntax designer.",
    "# Place in config/syntax/ and reopen your document.", "", "[syntax]",
    `name = ${tomlString(definition.name)}`,
    `extensions = [${definition.extensions.map(tomlString).join(", ")}]`,
  ];
  for (const rule of definition.rules) {
    lines.push("", "[[syntax.rules]]", `name = ${tomlString(rule.name)}`,
      `pattern = ${tomlString(rule.pattern)}`, `color = ${tomlString(rule.color)}`);
  }
  return `${lines.join("\n")}\n`;
}
export function fileName(name) {
  const slug = name.normalize("NFKD").replace(/[^a-zA-Z0-9]+/g, "_").replace(/^_+|_+$/g, "").toLowerCase();
  return `syntax_${slug.slice(0, 60) || "custom"}.toml`;
}
export function copyDefinition(definition) {
  return { name: definition.name, extensions: [...definition.extensions], rules: definition.rules.map(rule => ({ ...rule })) };
}
export function makeEngine(exports) {
  const encoder = new TextEncoder();
  const decoder = new TextDecoder();
  const send = (operation, text) => {
    const bytes = encoder.encode(text);
    if (bytes.length > MAX_DEFINITION_BYTES) throw new Error("Keep the definition under 64 KiB.");
    const pointer = exports.input_buffer(bytes.length);
    if (!pointer) throw new Error("The input is too large for the preview engine.");
    new Uint8Array(exports.memory.buffer, pointer, bytes.length).set(bytes);
    exports[operation]();
    const result = JSON.parse(decoder.decode(new Uint8Array(exports.memory.buffer, exports.output_ptr(), exports.output_len())));
    if (result.error) throw new Error(result.error);
    return result;
  };
  return {
    configure: source => send("configure_input", source),
    highlight: source => {
      if (encoder.encode(source).length > MAX_SAMPLE_BYTES) throw new Error("Keep the preview sample under 16 KiB.");
      return send("highlight_input", source);
    },
  };
}
