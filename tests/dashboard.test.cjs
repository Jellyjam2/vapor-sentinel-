const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const { execFileSync } = require('node:child_process');
const model = require('../dashboard/evidence.js');
const root = path.resolve(__dirname, '..');
const executable = process.env.VAPOR_SENTINEL_TEST_BIN || path.join(root, 'target', 'debug', `vapor_project${process.platform === 'win32' ? '.exe' : ''}`);
const env = { ...process.env, VAPOR_SENTINEL_ENABLE_ACTIONS: '0' };
for (const key of ['VAPOR_SENTINEL_CONFIG', 'VAPOR_SENTINEL_ONESHOT', 'VAPOR_SENTINEL_EXIT']) delete env[key];
const replay = (fixture) => execFileSync(executable, ['--replay', fixture], { cwd: root, env, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
const source = replay('tests/fixtures/memory-sequence.json');
const parseRows = (s = source) => s.trim().split('\n').map(JSON.parse);
const encode = (rows) => rows.map(JSON.stringify).join('\n');
const exampleContext = { window: {} };
vm.runInNewContext(fs.readFileSync(path.join(root, 'dashboard/example.js'), 'utf8'), exampleContext);
const example = exampleContext.window.VAPOR_EXAMPLE;

test('actual Rust records preserve qualification, values, and delivery outcomes', () => {
  const result = model.decode(source);
  assert.deepEqual(result.evaluations.map(r => r.evidence.state), ['Unknown', 'Anomalous', 'Anomalous', 'Normal', 'Normal']);
  assert.equal(result.streams.length, 1);
  assert.equal(result.evaluations[1].evidence.value, 10);
  assert.equal(model.outcome(result.evaluations.at(-1), result).reason, 'replay_disables_actions');
  assert.equal(model.outcome(result.evaluations[2], result), null);
});
test('summary distinguishes skipped delivery, missing outcomes, and scheduled intent', () => {
  const result = model.decode(source);
  assert.deepEqual(model.summarize(result.evaluations, result), {
    counts: {Normal:2, Anomalous:2, Unknown:1, Degraded:0},
    delivery: {delivered:0, skipped:2, failed:0, unrecorded:0}, scheduled:2, total:5
  });
  const incomplete = model.decode(encode(parseRows().filter(r => r.record_type === 'evaluation')));
  assert.equal(model.summarize(incomplete.evaluations, incomplete).delivery.unrecorded, 2);
});
test('bundled example decisions match a fresh Rust replay of its fixture', () => {
  const fresh = model.decode(replay('tests/fixtures/dashboard-sequence.json'));
  const bundled = model.decode(example);
  const decisions = r => r.evaluations.map(e => [e.evidence, e.plan, e.scheduled_plan, e.scheduling, e.policy_sha256]);
  assert.deepEqual(decisions(bundled), decisions(fresh));
  assert.equal(bundled.evaluations.length, 48);
  assert.equal(model.summarize(bundled.evaluations, bundled).counts.Anomalous, 6);
  assert.ok(bundled.evaluations.every(r => r.replay === true));
});
test('run, source, metric, unit, and policy boundaries split streams', () => {
  const first = parseRows()[0];
  const variants = [first];
  for (const [field, value] of [['run_id', 'another-run'], ['source_id', 'another-source'], ['unit', 'MiB'], ['policy_sha256', 'a'.repeat(64)]]) {
    const clone = structuredClone(first); clone[field] = value; clone.event_id = field; variants.push(clone);
  }
  const otherMetric = structuredClone(first); otherMetric.evidence.metric = 'SYSTEM_USED_MEMORY_MIB'; otherMetric.event_id = 'metric'; variants.push(otherMetric);
  assert.equal(model.decode(encode(variants)).streams.length, 6);
});
test('delivery association uses both run and event identity', () => {
  const rows = parseRows();
  const clone = structuredClone(rows.find(r => r.record_type === 'evaluation' && r.evidence.sequence === 2));
  clone.run_id = 'unrelated-run'; rows.push(clone);
  const result = model.decode(encode(rows));
  assert.equal(model.outcome(result.evaluations.at(-1), result), null);
  assert.equal(result.deliveries.size, 2);
});
test('filtering is case-insensitive and combines state with exact recording text', () => {
  const rows = model.decode(source).evaluations;
  assert.equal(model.filter(rows, 'Anomalous').length, 2);
  assert.equal(model.filter(rows, 'Anomalous', 'COOLDOWN').length, 1);
  assert.equal(model.filter(rows, 'Normal', 'LOW_AVAILABLE').length, 0);
  assert.equal(model.filter(rows, 'all', 'no match at all').length, 0);
});
test('filtered export retains matching outcomes in original record order and can be reimported', () => {
  const full = model.decode(source), selected = model.filter(full.evaluations, 'Anomalous');
  const exported = model.decode(model.exportRows(selected, full));
  assert.equal(exported.evaluations.length, 2); assert.equal(exported.deliveries.size, 1);
  assert.equal(model.outcome(exported.evaluations[0], exported).status, 'skipped');
  assert.deepEqual(exported.evaluations, selected);
  assert.deepEqual(model.decode(model.exportRows(full.evaluations, full)).records, full.records);
});
test('missing and malformed schemas fail with the original physical line number', () => {
  for (const input of ['', 'no JSON', 'null', '[]', '{}', '{"schema_version":2}']) assert.throws(() => model.decode(input));
  assert.throws(() => model.decode('\n\nnot JSON'), /line 3/);
  assert.equal(model.decode('\uFEFF' + source.replace(/\n/g, '\r\n')).evaluations.length, 5);
});
test('non-finite dates, unsafe integers, percentages, and malformed plans are rejected', () => {
  const first = parseRows()[0];
  const invalid = [
    r => { r.observed_at_ms = 8640000000000001; }, r => { r.observed_at_ms = -1; },
    r => { r.evidence.value = Number.MAX_SAFE_INTEGER + 1; }, r => { r.evidence.value = 101; },
    r => { r.evidence.sequence = 0; }, r => { r.evidence.sequence = 1.1; },
    r => { r.evidence.state = 'Healthy'; }, r => { r.evidence.reason = {}; },
    r => { r.evidence.deviation = {Increased:{delta:-1}}; }, r => { r.unit = {}; },
    r => { r.plan = {kind:'notify', message:{}}; }, r => { r.scheduled_plan = 'notify'; },
    r => { r.policy_sha256 = 'not-a-hash'; }, r => { r.replay = 'false'; }
  ];
  for (const change of invalid) { const row = structuredClone(first); change(row); assert.throws(() => model.decode(JSON.stringify(row))); }
});
test('duplicate evaluations and conflicting delivery results cannot silently overwrite evidence', () => {
  assert.throws(() => model.decode(source + JSON.stringify(parseRows()[0])), /Duplicate evaluation/);
  const delivery = parseRows().find(r => r.record_type === 'delivery');
  assert.throws(() => model.decode(source + JSON.stringify(delivery)), /Duplicate delivery/);
});
test('unmatched delivery records are surfaced without inflating stream counts', () => {
  const rows = parseRows(), unmatched = structuredClone(rows.find(r => r.record_type === 'delivery'));
  unmatched.delivery.event_id = 'missing-evaluation'; rows.push(unmatched);
  const result = model.decode(encode(rows));
  assert.equal(result.orphanDeliveries, 1); assert.equal(model.summarize(result.evaluations, result).delivery.skipped, 2);
});
test('UTF-8 bytes and total record limits are enforced before rendering', () => {
  assert.throws(() => model.decode('🎈'.repeat(Math.ceil(model.MAX_BYTES / 4) + 1)), /10 MiB/);
  const first = parseRows()[0], rows = [first];
  for (let i = 1; i < model.MAX_RECORDS; i++) rows.push({schema_version:1,record_type:'delivery',run_id:'r',delivery:{event_id:String(i),outcome:{status:'skipped'}}});
  assert.equal(model.decode(encode(rows)).records.length, model.MAX_RECORDS);
  rows.push({schema_version:1}); assert.throws(() => model.decode(encode(rows)), /too many records/);
});
test('literal hostile text remains data without normalization into markup', () => {
  const row = parseRows()[0]; row.evidence.reason = '<img src=x onerror=alert(1)>';
  assert.equal(model.decode(JSON.stringify(row)).evaluations[0].evidence.reason, row.evidence.reason);
});
test('deviation and metric labels preserve meaningful units and invalid-ordering states', () => {
  assert.equal(model.deviation({Decreased:{delta:4}}), 'Decreased by 4');
  assert.equal(model.deviation('SequenceGap'), 'Sequence gap');
  assert.equal(model.metricName('SYSTEM_AVAILABLE_MEMORY_PERCENT'), 'Available memory');
});
