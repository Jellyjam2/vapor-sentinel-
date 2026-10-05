// Exercise the actual browser script's event handlers against recorded Rust
// output. The small DOM port checks behavior; it is not a layout/browser test.
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const { execFileSync } = require('node:child_process');
const root = path.resolve(__dirname, '..');
const executable = process.env.VAPOR_SENTINEL_TEST_BIN || path.join(root, 'target', 'debug', `vapor_project${process.platform === 'win32' ? '.exe' : ''}`);
const env = { ...process.env, VAPOR_SENTINEL_ENABLE_ACTIONS: '0' };
for (const key of ['VAPOR_SENTINEL_CONFIG', 'VAPOR_SENTINEL_ONESHOT', 'VAPOR_SENTINEL_EXIT']) delete env[key];
const recording = execFileSync(executable, ['--replay', 'tests/fixtures/memory-sequence.json'], { cwd: root, env, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
function viewer() {
  const html = fs.readFileSync(path.join(root, 'dashboard/index.html'), 'utf8');
  const nodes = new Map([...html.matchAll(/id="([^"]+)"/g)].map(([,id]) => [id, { textContent: '', dataset: {}, disabled: false, handlers: {}, addEventListener(event,handler) { this.handlers[event] = handler; } }]));
  vm.runInNewContext(fs.readFileSync(path.join(root, 'dashboard/app.js'), 'utf8'), { document: { getElementById(id) { assert.ok(nodes.has(id), `Missing HTML element ${id}`); return nodes.get(id); } } });
  return { nodes, load(source, size = Buffer.byteLength(source)) { return nodes.get('evidence-file').handlers.change({ target: { files: [{ name: 'evidence.jsonl', size, text: async () => source }] } }); } };
}
test('actual runtime schema, navigation and delivery results render', async () => {
  const { nodes, load } = viewer(); await load(recording);
  assert.equal(nodes.get('state').textContent,'NORMAL');
  assert.equal(nodes.get('position').textContent,'5 of 5');
  assert.match(nodes.get('delivery').textContent,/replay_disables_actions/);
  assert.equal(nodes.get('next').disabled,true);
  for (let index=0; index<3; index++) nodes.get('previous').handlers.click();
  assert.equal(nodes.get('state').textContent,'ANOMALOUS');
  assert.equal(nodes.get('value').textContent,'10');
  assert.equal(nodes.get('unit').textContent,'percent');
  nodes.get('previous').handlers.click();
  assert.equal(nodes.get('state').textContent,'UNKNOWN');
  assert.equal(nodes.get('previous').disabled,true);
});
test('invalid, future-schema and oversized files cannot replace the displayed recording', async () => {
  const { nodes, load }=viewer(); await load(recording);
  for (const input of ['not JSON', JSON.stringify({schema_version:2}), '']) {
    await load(input); assert.match(nodes.get('status').textContent,/Could not load/); assert.equal(nodes.get('state').textContent,'NORMAL');
  }
  await load(recording,10*1024*1024+1); assert.match(nodes.get('status').textContent,/exceeds 10 MiB/);
});
test('untrusted display strings are assigned as text', async () => {
  const { nodes, load }=viewer();
  const rows=recording.trim().split('\n').map(JSON.parse);
  for(const row of rows) if(row.record_type==='evaluation') row.evidence.reason='<img src=x onerror=alert(1)>';
  await load(rows.map(JSON.stringify).join('\n'));
  assert.equal(nodes.get('reason').textContent,'<img src=x onerror=alert(1)>');
});
test('a slower previous file read cannot overwrite a newer selection', async () => {
  const { nodes, load }=viewer(); let finish;
  const promise = new Promise(resolve => { finish=resolve; });
  const first=nodes.get('evidence-file').handlers.change({target:{files:[{name:'old.jsonl',size:100,text:()=>promise}]}});
  await load(recording);
  finish('not JSON'); await first;
  assert.match(nodes.get('status').textContent,/Loaded 5 observations/);
});
