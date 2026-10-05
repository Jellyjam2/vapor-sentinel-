/* Version 1 JSONL adapter. File contents are treated as data, never as markup. */
(function () {
  "use strict";
  const limit = 10 * 1024 * 1024;
  const states = new Set(["Normal", "Degraded", "Anomalous", "Unknown"]);
  let evaluations = [], deliveries = new Map(), position = 0, loadVersion = 0;
  const element = (id) => document.getElementById(id);
  const text = (id, value) => { element(id).textContent = String(value); };
  const key = (run, event) => JSON.stringify([run, event]);
  function decode(source) {
    const lines = source.split(/\r?\n/).filter((line) => line.trim());
    if (lines.length > 20000) throw new Error("Recording has too many records (maximum 20000).");
    const entries = [], results = new Map();
    for (let index = 0; index < lines.length; index += 1) {
      let row;
      try { row = JSON.parse(lines[index]); } catch { throw new Error(`Invalid JSON on line ${index + 1}.`); }
      if (!row || row.schema_version !== 1) throw new Error(`Unsupported schema on line ${index + 1}.`);
      if (row.record_type === "evaluation") {
        const e = row.evidence;
        if (!e || !states.has(e.state) || typeof e.metric !== "string" ||
            typeof e.reason !== "string" || typeof row.run_id !== "string" || typeof row.event_id !== "string" ||
            !Number.isSafeInteger(e.sequence) || e.sequence < 1 || !Number.isSafeInteger(e.value) || e.value < 0 ||
            !Number.isSafeInteger(row.observed_at_ms)) throw new Error(`Invalid evaluation on line ${index + 1}.`);
        entries.push(row);
      } else if (row.record_type === "delivery") {
        const d = row.delivery;
        if (!d || typeof d.event_id !== "string" || typeof row.run_id !== "string" || !d.outcome ||
            !["delivered", "skipped", "failed"].includes(d.outcome.status)) throw new Error(`Invalid delivery on line ${index + 1}.`);
        results.set(key(row.run_id, d.event_id), d.outcome);
      } else throw new Error(`Unknown record type on line ${index + 1}.`);
    }
    if (!entries.length) throw new Error("No evaluation records in this file.");
    return { evaluations: entries, deliveries: results };
  }
  function render() {
    const row = evaluations[position], e = row.evidence;
    text("state", e.state.toUpperCase()); element("state").dataset.state = e.state;
    text("sequence", e.sequence); text("reason", e.reason); text("metric", e.metric); text("value", e.value);
    text("unit", row.unit || ""); text("timestamp", new Date(row.observed_at_ms).toLocaleString());
    text("deviation", typeof e.deviation === "string" ? e.deviation : JSON.stringify(e.deviation));
    text("source", row.source_id || "Not recorded"); text("run", row.run_id); text("policy", row.policy_sha256 || "Not recorded");
    text("mode", row.replay ? "Replay (actions disabled)" : "System observation");
    text("plan", row.plan?.kind || "Not recorded"); text("scheduling", row.scheduling || "Not recorded");
    text("message", row.scheduled_plan?.message || row.plan?.message || "No notification message");
    const delivery = deliveries.get(key(row.run_id, row.event_id));
    text("delivery", delivery ? `${delivery.status}${delivery.reason ? ": " + delivery.reason : ""}` : "No delivery result recorded");
    text("position", `${position + 1} of ${evaluations.length}`);
    element("previous").disabled = position === 0; element("next").disabled = position === evaluations.length - 1;
  }
  element("evidence-file").addEventListener("change", async (event) => {
    const version = ++loadVersion;
    const file = event.target.files[0]; if (!file) return;
    try {
      if (file.size > limit) throw new Error("File exceeds 10 MiB. Export a smaller recording.");
      const parsed = decode(await file.text());
      if (version !== loadVersion) return;
      evaluations = parsed.evaluations; deliveries = parsed.deliveries; position = evaluations.length - 1;
      render(); text("status", `Loaded ${evaluations.length} observations from ${file.name}.`);
    } catch (error) {
      if (version === loadVersion) text("status", `Could not load recording: ${error.message}${evaluations.length ? " Previous recording remains displayed." : ""}`);
    }
  });
  element("previous").addEventListener("click", () => { if (position > 0) { position -= 1; render(); } });
  element("next").addEventListener("click", () => { if (position + 1 < evaluations.length) { position += 1; render(); } });
})();
