/* Pure recording model, shared by the browser and Node regression tests. */
(function (root, factory) {
  const api = factory();
  if (typeof module === "object" && module.exports) module.exports = api;
  else root.VaporEvidence = api;
})(typeof globalThis !== "undefined" ? globalThis : this, function () {
  "use strict";
  const MAX_BYTES = 10 * 1024 * 1024, MAX_RECORDS = 20000;
  const STATES = ["Normal", "Anomalous", "Unknown", "Degraded"];
  const key = (run, event) => JSON.stringify([run, event]);
  const object = (v) => v !== null && typeof v === "object" && !Array.isArray(v);
  const string = (v, max = 4096) => typeof v === "string" && v.length <= max;
  const identity = (v) => string(v, 512) && v.length > 0;
  const integer = (v) => Number.isSafeInteger(v) && v >= 0;
  const timestamp = (v) => integer(v) && v <= 8640000000000000;
  function validPlan(plan) {
    return object(plan) && ((plan.kind === "no_action" && plan.message === undefined) ||
      (["notify", "recovered"].includes(plan.kind) && string(plan.message)));
  }
  function validDeviation(value) {
    if (["NoBaseline", "Unchanged", "DuplicateSequence", "OutOfOrderSequence", "MetricMismatch", "SequenceGap"].includes(value)) return true;
    if (!object(value) || Object.keys(value).length !== 1) return false;
    const delta = value.Increased || value.Decreased;
    return object(delta) && integer(delta.delta);
  }
  function decode(source) {
    if (typeof source !== "string") throw new Error("The recording must contain text.");
    if (new TextEncoder().encode(source).byteLength > MAX_BYTES) throw new Error("File exceeds 10 MiB. Export a smaller recording.");
    const lines = source.replace(/^\uFEFF/, "").split(/\r?\n/);
    const evaluations = [], records = [], deliveries = new Map(), identities = new Set(), groups = new Map();
    for (let index = 0; index < lines.length; index += 1) {
      if (!lines[index].trim()) continue;
      if (records.length === MAX_RECORDS) throw new Error("Recording has too many records (maximum 20000).");
      let row;
      try { row = JSON.parse(lines[index]); } catch { throw new Error(`Invalid JSON on line ${index + 1}.`); }
      const fail = (what) => { throw new Error(`${what} on line ${index + 1}.`); };
      if (!object(row) || row.schema_version !== 1) fail("Unsupported schema");
      if (!identity(row.run_id)) fail("Invalid run identity");
      if (row.record_type === "evaluation") {
        const e = row.evidence;
        if (!object(e) || !STATES.includes(e.state) || !identity(e.metric) || !string(e.reason) ||
            !identity(row.event_id) || !integer(e.sequence) || e.sequence < 1 || !integer(e.value) ||
            !timestamp(row.observed_at_ms) || !validDeviation(e.deviation)) fail("Invalid evaluation");
        if (row.unit !== undefined && !string(row.unit, 64)) fail("Invalid unit");
        if (row.source_id !== undefined && !identity(row.source_id)) fail("Invalid source identity");
        if (row.policy_sha256 !== undefined && !(typeof row.policy_sha256 === "string" && /^[a-f0-9]{64}$/i.test(row.policy_sha256))) fail("Invalid policy hash");
        if (row.replay !== undefined && typeof row.replay !== "boolean") fail("Invalid replay flag");
        if (row.plan !== undefined && !validPlan(row.plan)) fail("Invalid policy plan");
        if (row.scheduled_plan !== undefined && !validPlan(row.scheduled_plan)) fail("Invalid scheduled plan");
        if (row.scheduling !== undefined && !string(row.scheduling, 128)) fail("Invalid scheduling value");
        if (e.matched_messages !== undefined && (!Array.isArray(e.matched_messages) || e.matched_messages.length > 64 || !e.matched_messages.every((s) => string(s)))) fail("Invalid matched messages");
        if (row.unit === "percent" && e.value > 100) fail("Percentage exceeds 100");
        const id = key(row.run_id, row.event_id);
        if (identities.has(id)) fail("Duplicate evaluation identity");
        identities.add(id); evaluations.push(row);
        const streamKey = JSON.stringify([row.run_id, row.source_id || "", e.metric, row.unit || "", row.policy_sha256 || "", row.replay]);
        if (!groups.has(streamKey)) groups.set(streamKey, { id: streamKey, run: row.run_id, source: row.source_id || "Not recorded", metric: e.metric, unit: row.unit || "", replay: row.replay, rows: [] });
        groups.get(streamKey).rows.push(row);
      } else if (row.record_type === "delivery") {
        const d = row.delivery;
        if (!object(d) || !identity(d.event_id) || !object(d.outcome) || !["delivered", "skipped", "failed"].includes(d.outcome.status) ||
            (d.outcome.reason !== undefined && !string(d.outcome.reason)) ||
            (d.plan !== undefined && !validPlan(d.plan)) ||
            (row.recorded_at_ms !== undefined && !timestamp(row.recorded_at_ms))) fail("Invalid delivery");
        const id = key(row.run_id, d.event_id);
        if (deliveries.has(id)) fail("Duplicate delivery identity");
        deliveries.set(id, row);
      } else fail("Unknown record type");
      records.push(row);
    }
    if (!evaluations.length) throw new Error("No evaluation records in this file.");
    const orphanDeliveries = [...deliveries.keys()].filter((id) => !identities.has(id)).length;
    return { evaluations, deliveries, records, streams: [...groups.values()], orphanDeliveries };
  }
  function outcome(row, recording) { return recording.deliveries.get(key(row.run_id, row.event_id))?.delivery.outcome || null; }
  function summarize(rows, recording) {
    const counts = Object.fromEntries(STATES.map((s) => [s, 0]));
    const delivery = { delivered: 0, skipped: 0, failed: 0, unrecorded: 0 };
    let scheduled = 0;
    for (const row of rows) {
      counts[row.evidence.state] += 1;
      const result = outcome(row, recording);
      if (result) delivery[result.status] += 1;
      else if (["notify", "recovered"].includes(row.scheduled_plan?.kind)) delivery.unrecorded += 1;
      if (["notify", "recovered"].includes(row.scheduled_plan?.kind)) scheduled += 1;
    }
    return { counts, delivery, scheduled, total: rows.length };
  }
  function filter(rows, state = "all", query = "") {
    const search = query.trim().toLowerCase();
    return rows.filter((row) => (state === "all" || row.evidence.state === state) && (!search ||
      [row.event_id, row.evidence.sequence, row.evidence.state, row.evidence.reason, row.evidence.metric,
        row.plan?.message, row.scheduled_plan?.message, row.scheduling].join(" ").toLowerCase().includes(search)));
  }
  function exportRows(rows, recording) {
    const wanted = new Set(rows.map((row) => key(row.run_id, row.event_id)));
    return recording.records.filter((row) => wanted.has(key(row.run_id, row.record_type === "evaluation" ? row.event_id : row.delivery.event_id)))
      .map((row) => JSON.stringify(row)).join("\n") + "\n";
  }
  function metricName(metric) { return ({ SYSTEM_AVAILABLE_MEMORY_PERCENT: "Available memory", SYSTEM_USED_MEMORY_MIB: "Used memory" })[metric] || metric; }
  function deviation(value) {
    if (value?.Increased) return `Increased by ${value.Increased.delta}`;
    if (value?.Decreased) return `Decreased by ${value.Decreased.delta}`;
    return ({ NoBaseline: "Baseline unavailable", Unchanged: "Unchanged", DuplicateSequence: "Duplicate sequence", OutOfOrderSequence: "Out-of-order sequence", MetricMismatch: "Metric mismatch", SequenceGap: "Sequence gap" })[value] || "Not recorded";
  }
  return { MAX_BYTES, MAX_RECORDS, STATES, key, decode, outcome, summarize, filter, exportRows, metricName, deviation };
});
