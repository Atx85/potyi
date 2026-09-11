import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';
import { exportToml, makeEngine, fileName } from '../../docs/designer/model.mjs';
const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const bytes = await readFile(resolve(root, 'docs/designer/engine.wasm'));
const { instance } = await WebAssembly.instantiate(bytes, {});
const engine = makeEngine(instance.exports);
const presets = JSON.parse(await readFile(resolve(root, 'docs/designer/presets.json'), 'utf8'));
let count = 0;
for (const preset of presets) {
  const exported = exportToml(preset.definition);
  assert.deepEqual(engine.configure(exported), preset.definition, `TOML round trip: ${preset.filename}`);
  const source = preset.sample;
  const result = engine.highlight(source);
  const lines = source.split('\n');
  assert.equal(result.lines.length, lines.length);
  for (let i = 0; i < result.lines.length; i++) {
    const bytes = new TextEncoder().encode(lines[i]);
    let previous = 0;
    for (const [start, end, color] of result.lines[i]) {
      assert(start >= previous && end > start && end <= bytes.length);
      assert.match(color, /^#[0-9a-f]{6}$/);
      new TextDecoder('utf-8', { fatal: true }).decode(bytes.slice(start, end));
      previous = end;
    }
  }
  if (preset.filename !== 'syntax_custom.toml') {
    const original = await readFile(resolve(root, 'config/syntax', preset.filename), 'utf8');
    assert.deepEqual(engine.configure(original), preset.definition, `Import bundled TOML: ${preset.filename}`);
  }
  count++;
}
const python = structuredClone(presets.find(p => p.definition.name === 'Python').definition);
python.name = 'Quotes " and Unicode: café';
python.rules[0].name = 'Comment\twith\ncontrols\u007f';
assert.deepEqual(engine.configure(exportToml(python)), python);
const unicode = '# café 🐈\nname = "héllo"';
const spans = engine.highlight(unicode).lines;
assert.equal(spans[0][0][1], new TextEncoder().encode('# café 🐈').length);
for (const pattern of ['(?=test)', '(test)\\1', '(']) {
  const invalid = structuredClone(python); invalid.rules[0].pattern = pattern;
  assert.throws(() => engine.configure(exportToml(invalid)), /Rule 1/);
}
assert.throws(() => engine.configure('not toml'), /Invalid TOML/);
engine.configure(exportToml(python));
assert.throws(() => engine.highlight('é'.repeat(9000)), /16 KiB/);
assert.throws(() => engine.configure('x'.repeat(65537)), /64 KiB/);
assert.equal(engine.highlight('x\n'.repeat(250)).lines.length, 200);
const dense = { name:'Dense', extensions:['dense'], rules:[{name:'all', pattern:'.', color:'#FFFFFF'}] };
engine.configure(exportToml(dense));
assert.equal(engine.highlight('x'.repeat(12000)).lines[0].length, 1024);
assert.equal(fileName('../../Unsafe Name'), 'syntax_unsafe_name.toml');
const downloaded = await readFile(resolve(root, 'tools/syntax-designer/fixtures/browser-export-python.toml'), 'utf8');
const downloadedDefinition = engine.configure(downloaded);
assert.equal(downloadedDefinition.rules[0].color, '#e879f9');
assert.equal(exportToml(downloadedDefinition), downloaded, 'Actual browser download matches the exporter');
console.log(`Passed: ${count} TOML round trips, original imports, sample spans, UTF-8, unsupported regex, invalid TOML, and resource limits.`);
