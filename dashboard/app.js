/* Local-only evidence workspace. Imported content is always rendered as text. */
(function () {
  "use strict";
  const model = window.VaporEvidence;
  const $ = (id) => document.getElementById(id);
  const text = (id, value) => { $(id).textContent = String(value); };
  const node = (tag, className, value) => {
    const result = document.createElement(tag);
    if (className) result.className = className;
    if (value !== undefined) result.textContent = String(value);
    return result;
  };
  const svgNode = (tag, attributes = {}) => {
    const result = document.createElementNS("http://www.w3.org/2000/svg", tag);
    for (const [name, value] of Object.entries(attributes)) result.setAttribute(name, String(value));
    return result;
  };
  const icon = (name, className = "") => {
    const svg = svgNode("svg", { "aria-hidden": "true", class: className });
    svg.append(svgNode("use", { href: `#i-${name}` }));
    return svg;
  };
  const badge = (value, label = value) => {
    const result = node("span", "badge", label);
    result.dataset.state = value;
    return result;
  };
  const pretty = (s) => s ? String(s).replace(/_/g, " ").replace(/^./, (c) => c.toUpperCase()) : "Not recorded";
  const unit = (value) => value === "percent" ? "%" : value || "";
  const format = (value) => Number(value).toLocaleString();
  const time = (value) => new Date(value).toLocaleString(undefined, { dateStyle: "medium", timeStyle: "medium" });
  const planName = (plan) => ({ no_action: "No action", notify: "Notify", recovered: "Recovery" })[plan?.kind] || "Not recorded";
  const PAGE_SIZE = 8, DELIVERY_PAGE_SIZE = 5;
  let recording = null, stream = null, rows = [], filtered = [], selected = null;
  let stateFilter = "all", page = 0, deliveryPage = 0, loadVersion = 0, dragDepth = 0;
  let example = false, filename = "", chartScale = 100;
  function status(message, error = false) {
    text("status", message);
    $("status").classList.toggle("error", error);
  }
  function resetFilters() {
    stateFilter = "all"; $("search").value = ""; page = 0;
    for (const button of document.querySelectorAll("[data-filter]")) button.setAttribute("aria-pressed", String(button.dataset.filter === stateFilter));
  }
  function accept(parsed, name, isExample) {
    recording = parsed; filename = name; example = isExample;
    const options = recording.streams.map((s, i) => {
      const option = node("option", "", `${i + 1}. ${s.source} · ${model.metricName(s.metric)} · ${s.rows.length} observations`);
      option.value = String(i); return option;
    });
    $("stream").replaceChildren(...options); $("stream").disabled = false;
    $("stream").value = String(recording.streams.length - 1);
    text("recording-name", name); $("recording-name").title = name;
    $("clear").disabled = false;
    chooseStream();
    const orphan = parsed.orphanDeliveries ? ` ${parsed.orphanDeliveries} unmatched delivery records are excluded from stream summaries.` : "";
    status(`${isExample ? "Example recording · " : ""}Loaded ${format(parsed.evaluations.length)} observations across ${parsed.streams.length} stream${parsed.streams.length === 1 ? "" : "s"}.${orphan}`);
  }
  async function loadFile(file) {
    if (!file) return;
    const version = ++loadVersion;
    status(`Opening ${file.name}…`); $("main").setAttribute("aria-busy", "true");
    try {
      if (file.size > model.MAX_BYTES) throw new Error("File exceeds 10 MiB. Export a smaller recording.");
      const source = await file.text();
      if (version !== loadVersion) return;
      const parsed = model.decode(source);
      if (version === loadVersion) accept(parsed, file.name, false);
    } catch (error) {
      if (version === loadVersion) status(`Could not load recording: ${error.message}${recording ? " Previous recording remains displayed." : ""}`, true);
    } finally {
      if (version === loadVersion) $("main").removeAttribute("aria-busy");
    }
  }
  function chooseStream() {
    stream = recording.streams[Number($("stream").value)]; rows = stream.rows;
    resetFilters(); filtered = rows; selected = rows.at(-1); deliveryPage = 0;
    text("recording-mode", example ? "EXAMPLE · REPLAY" : stream.replay === true ? "REPLAY RECORDING" : stream.replay === false ? "SYSTEM RECORDING" : "RECORDED EVIDENCE");
    $("recording-mode").classList.toggle("example", example);
    renderSummary(); renderChart(); renderTable(); renderInspector(); renderDeliveries();
  }
  function renderSummary() {
    const result = model.summarize(rows, recording), last = rows.at(-1), value = last.evidence;
    text("latest-state", value.state); $("latest-state").dataset.state = value.state;
    text("latest-state-note", `Observation #${format(value.sequence)} · ${stream.replay === true ? "Replay" : "Recorded"}`);
    text("summary-metric", model.metricName(stream.metric).toUpperCase());
    text("latest-value", format(value.value)); text("latest-unit", unit(stream.unit));
    text("value-note", `Latest of ${format(rows.length)} observations`);
    text("anomaly-count", format(result.counts.Anomalous));
    text("anomaly-note", `${(result.counts.Anomalous / rows.length * 100).toFixed(1)}% of this stream`);
    const deliveryCount = result.delivery.delivered + result.delivery.skipped + result.delivery.failed;
    text("delivery-count", format(deliveryCount));
    text("delivery-note", `${result.delivery.delivered} delivered · ${result.delivery.failed} failed · ${result.delivery.skipped} skipped`);
    text("nav-count", format(rows.length)); text("nav-deliveries", format(deliveryCount));
    text("observation-count", format(rows.length));
    const colors = ["var(--green)", "var(--red)", "var(--purple)", "var(--yellow)"];
    let at = 0;
    const stops = model.STATES.map((s, i) => {
      text(`count-${s.toLowerCase()}`, `${format(result.counts[s])} / ${(result.counts[s] / rows.length * 100).toFixed(0)}%`);
      const start = at; at += result.counts[s] / rows.length * 100;
      return `${colors[i]} ${start}% ${at}%`;
    });
    $("distribution").style.background = `conic-gradient(${stops.join(",")})`;
    text("chart-unit", stream.unit || "VALUE");
    text("chart-description", `${model.metricName(stream.metric)} · ${format(rows.length)} observations`);
  }
  const plot = { left: 48, right: 780, top: 17, bottom: 216 };
  const coordinates = (row, index) => ({
    x: rows.length <= 1 ? (plot.left + plot.right) / 2 : plot.left + index / (rows.length - 1) * (plot.right - plot.left),
    y: plot.bottom - row.evidence.value / chartScale * (plot.bottom - plot.top)
  });
  function renderChart() {
    const chart = $("timeline"); chart.replaceChildren();
    let maximum = 1;
    for (const row of rows) maximum = Math.max(maximum, row.evidence.value);
    chartScale = stream?.unit === "percent" ? 100 : Math.max(1, Math.ceil(maximum / Math.pow(10, Math.floor(Math.log10(maximum)))) * Math.pow(10, Math.floor(Math.log10(maximum))));
    const defs = svgNode("defs"), gradient = svgNode("linearGradient", { id: "area-fill", x1: 0, y1: 0, x2: 0, y2: 1 });
    gradient.append(svgNode("stop", { offset: "0%", "stop-color": "#d3bd91", "stop-opacity": ".17" }), svgNode("stop", { offset: "100%", "stop-color": "#d3bd91", "stop-opacity": ".005" }));
    defs.append(gradient); chart.append(defs);
    for (let i = 0; i <= 4; i += 1) {
      const y = plot.top + i / 4 * (plot.bottom - plot.top);
      chart.append(svgNode("line", { x1: plot.left, y1: y, x2: plot.right, y2: y, class: "chart-grid-line" }));
      const label = svgNode("text", { x: plot.left - 12, y: y + 4, "text-anchor": "end", class: "chart-axis-label" });
      label.textContent = rows.length ? new Intl.NumberFormat(undefined, { notation: "compact", maximumFractionDigits: 1 }).format(chartScale * (1 - i / 4)) : "—";
      chart.append(label);
    }
    $("chart-empty").hidden = rows.length > 0;
    $("timeline-index").disabled = rows.length === 0;
    $("timeline-index").max = String(Math.max(0, rows.length - 1));
    if (!rows.length) {
      chart.setAttribute("aria-label", "No observations loaded");
      text("chart-start", "—"); text("chart-end", "—"); return;
    }
    // Never join a sequence gap or reversed/duplicate observation with a line.
    let segments = [], segment = [];
    rows.forEach((row, i) => {
      if (i && row.evidence.sequence !== rows[i - 1].evidence.sequence + 1) { segments.push(segment); segment = []; }
      const point = coordinates(row, i); segment.push(`${point.x.toFixed(2)} ${point.y.toFixed(2)}`);
    });
    if (segment.length) segments.push(segment);
    const line = segments.map((s) => `M${s.join(" L")}`).join(" ");
    const area = segments.map((s) => `M${s[0].split(" ")[0]} ${plot.bottom} L${s.join(" L")} L${s.at(-1).split(" ")[0]} ${plot.bottom} Z`).join(" ");
    chart.append(svgNode("path", { d: area, class: "chart-area" }), svgNode("path", { d: line, class: "chart-line" }));
    // The full series is drawn; a bounded set of flags keeps the DOM small.
    let flags = 0;
    rows.forEach((row, i) => {
      if (row.evidence.state === "Anomalous" && flags < 64) {
        const point = coordinates(row, i); chart.append(svgNode("circle", { cx: point.x, cy: point.y, r: 3.7, class: "chart-alert" })); flags += 1;
      }
    });
    chart.append(svgNode("line", { id: "chart-marker-line", class: "chart-marker-line", y1: plot.top, y2: plot.bottom }), svgNode("circle", { id: "chart-marker", r: 5, class: "chart-marker" }));
    chart.setAttribute("aria-label", `${model.metricName(stream.metric)}, ${rows.length} observations in recording order. Use the slider or evidence table to inspect exact values. Lines break at invalid sequence boundaries.`);
    text("chart-start", `#${rows[0].evidence.sequence}`); text("chart-end", `#${rows.at(-1).evidence.sequence}`);
    renderMarker();
  }
  function renderMarker() {
    const circle = $("chart-marker"), line = $("chart-marker-line");
    if (!circle) return;
    circle.hidden = !selected; line.hidden = !selected;
    circle.style.display = selected ? "" : "none"; line.style.display = selected ? "" : "none";
    if (!selected) { text("chart-selection", "No matching observation selected"); return; }
    const index = rows.indexOf(selected), point = coordinates(selected, index);
    circle.setAttribute("cx", point.x); circle.setAttribute("cy", point.y);
    line.setAttribute("x1", point.x); line.setAttribute("x2", point.x);
    $("timeline-index").value = String(index);
    $("timeline-index").setAttribute("aria-valuetext", `Observation ${selected.evidence.sequence}: ${selected.evidence.value} ${unit(stream.unit)}, ${selected.evidence.state}`);
    text("chart-selection", `#${selected.evidence.sequence} · ${format(selected.evidence.value)}${unit(stream.unit) === "%" ? "%" : " " + unit(stream.unit)} · ${selected.evidence.state}`);
  }
  function renderTable() {
    const body = $("evidence-rows"); body.replaceChildren();
    const newest = filtered.slice().reverse(), start = page * PAGE_SIZE;
    for (const row of newest.slice(start, start + PAGE_SIZE)) {
      const tr = node("tr", row === selected ? "selected" : ""); tr.dataset.index = String(rows.indexOf(row));
      const sequence = node("td"), button = node("button", "sequence-button", `#${String(row.evidence.sequence).padStart(4, "0")}`);
      button.setAttribute("aria-label", `Inspect observation ${row.evidence.sequence}`); button.setAttribute("aria-pressed", String(row === selected)); sequence.append(button);
      const state = node("td"); state.append(badge(row.evidence.state));
      const value = node("td", "table-value", format(row.evidence.value)); value.append(node("small", "", unit(stream.unit)));
      const decision = node("td", "", ["notify", "recovered"].includes(row.scheduled_plan?.kind) ? planName(row.scheduled_plan) : pretty(row.scheduling));
      const arrow = node("td"); arrow.append(icon("arrow-up", "inspect-arrow"));
      tr.append(sequence, state, value, decision, arrow); body.append(tr);
    }
    $("table-empty").hidden = filtered.length > 0;
    text("table-empty-title", recording ? "No matching observations" : "No evidence yet");
    text("table-empty-description", recording ? "Try another filter or search term." : "Your observations will appear here.");
    text("table-range", filtered.length ? `${start + 1}–${Math.min(start + PAGE_SIZE, filtered.length)} of ${format(filtered.length)} · newest first` : "0 observations");
    text("page-label", filtered.length ? `${page + 1} / ${Math.ceil(filtered.length / PAGE_SIZE)}` : "—");
    $("page-previous").disabled = page === 0; $("page-next").disabled = start + PAGE_SIZE >= filtered.length;
    $("export").disabled = filtered.length === 0;
  }
  function renderInspector() {
    if (!selected) {
      for (const id of ["selected-sequence", "timestamp", "deviation", "plan", "scheduling", "delivery", "source", "run", "event", "policy", "mode", "inspector-value"]) text(id, "—");
      text("state", "No selection"); delete $("state").dataset.state;
      text("reason", "Select an observation to understand the decision."); text("message", "No observation selected.");
      text("position", "No observation selected"); $("previous").disabled = true; $("next").disabled = true; renderMarker(); return;
    }
    const e = selected.evidence, result = model.outcome(selected, recording);
    text("selected-sequence", `#${String(e.sequence).padStart(4, "0")}`);
    text("state", e.state); $("state").dataset.state = e.state;
    text("inspector-value", `${format(e.value)}${unit(stream.unit) === "%" ? "%" : " " + unit(stream.unit)}`);
    text("reason", e.reason); text("timestamp", time(selected.observed_at_ms)); text("deviation", model.deviation(e.deviation));
    text("plan", planName(selected.plan)); text("scheduling", pretty(selected.scheduling));
    text("delivery", result ? `${pretty(result.status)}${result.reason ? " · " + pretty(result.reason) : ""}` : "No result recorded");
    text("message", selected.scheduled_plan?.message || selected.plan?.message || "No notification message");
    text("source", selected.source_id || "Not recorded"); text("run", selected.run_id); text("event", selected.event_id);
    text("policy", selected.policy_sha256 || "Not recorded"); text("mode", selected.replay === true ? "Replay · actions disabled" : selected.replay === false ? "System observation" : "Not recorded");
    const index = filtered.indexOf(selected);
    text("position", `${index + 1} of ${filtered.length} in view`);
    $("previous").disabled = index <= 0; $("next").disabled = index < 0 || index >= filtered.length - 1;
    renderMarker();
  }
  function deliveryRows() {
    return rows.filter((row) => model.outcome(row, recording) || ["notify", "recovered"].includes(row.scheduled_plan?.kind)).reverse();
  }
  function renderDeliveries() {
    const list = $("delivery-list"); list.replaceChildren();
    const entries = recording ? deliveryRows() : [], start = deliveryPage * DELIVERY_PAGE_SIZE;
    if (!entries.length) {
      const empty = node("div", "delivery-empty"); empty.append(icon("send"));
      const copy = node("div"); copy.append(node("strong", "", "No delivery activity recorded"), node("p", "", "Scheduled notifications and their outcomes appear here.")); empty.append(copy); list.append(empty);
    }
    for (const row of entries.slice(start, start + DELIVERY_PAGE_SIZE)) {
      const record = recording.deliveries.get(model.key(row.run_id, row.event_id)), result = record?.delivery.outcome;
      const item = node("div", "delivery-item"), symbol = node("span", "delivery-icon"); symbol.append(icon("send"));
      const details = node("div"), title = planName(record?.delivery.plan || row.scheduled_plan);
      details.append(node("div", "delivery-title", `${title} · Observation #${row.evidence.sequence}`), node("div", "delivery-meta", record?.recorded_at_ms !== undefined ? time(record.recorded_at_ms) : "No delivery timestamp recorded"));
      const detail = result?.reason ? pretty(result.reason) : result?.status === "delivered" ? "Destination accepted the notification" : "No additional detail recorded";
      const reason = node("div", "delivery-reason", result ? detail : "A plan exists, but no outcome is in this recording.");
      item.append(symbol, details, reason, badge(result?.status || "unrecorded", result ? pretty(result.status) : "Unrecorded")); list.append(item);
    }
    const summary = recording ? model.summarize(rows, recording) : null;
    text("delivery-description", summary?.delivery.unrecorded ? `${summary.delivery.unrecorded} scheduled notification(s) have no recorded outcome. This does not establish successful delivery.` : "A notification plan is intent. A delivery record tells you what happened.");
    text("delivery-range", entries.length ? `${start + 1}–${Math.min(start + DELIVERY_PAGE_SIZE, entries.length)} of ${entries.length} notifications` : "No delivery records");
    text("delivery-page-label", entries.length ? `${deliveryPage + 1} / ${Math.ceil(entries.length / DELIVERY_PAGE_SIZE)}` : "—");
    $("delivery-previous").disabled = deliveryPage === 0; $("delivery-next").disabled = start + DELIVERY_PAGE_SIZE >= entries.length;
  }
  function select(row, alignPage = false) {
    selected = row;
    if (alignPage && row) page = Math.floor((filtered.length - 1 - filtered.indexOf(row)) / PAGE_SIZE);
    renderTable(); renderInspector();
  }
  function applyFilter() {
    filtered = model.filter(rows, stateFilter, $("search").value); page = 0;
    for (const button of document.querySelectorAll("[data-filter]")) button.setAttribute("aria-pressed", String(button.dataset.filter === stateFilter));
    select(filtered.at(-1) || null);
  }
  function chartSelect(index) {
    if (!rows.length) return;
    const row = rows[Math.max(0, Math.min(rows.length - 1, index))];
    if (!filtered.includes(row)) { resetFilters(); filtered = rows; }
    select(row, true);
  }
  function clear() {
    ++loadVersion; recording = null; stream = null; rows = []; filtered = []; selected = null; example = false; filename = ""; deliveryPage = 0;
    resetFilters(); $("main").removeAttribute("aria-busy"); $("evidence-file").value = "";
    $("stream").replaceChildren(node("option", "", "No evidence stream")); $("stream").disabled = true; $("clear").disabled = true;
    text("recording-name", "No recording open"); $("recording-name").removeAttribute("title"); text("recording-mode", "LOCAL ONLY"); $("recording-mode").classList.remove("example");
    text("latest-state", "Awaiting data"); delete $("latest-state").dataset.state; text("latest-state-note", "Your recording starts the story");
    text("summary-metric", "MEMORY OBSERVATION"); text("latest-value", "—"); text("latest-unit", ""); text("value-note", "No measurement loaded");
    text("anomaly-count", "—"); text("anomaly-note", "Counted from recorded states"); text("delivery-count", "—"); text("delivery-note", "Outcomes, not assumptions");
    text("nav-count", "0"); text("nav-deliveries", "0"); text("observation-count", "—");
    for (const s of model.STATES) text(`count-${s.toLowerCase()}`, "—");
    $("distribution").style.background = ""; text("chart-unit", "NO DATA"); text("chart-description", "A measured view of your selected stream.");
    text("chart-selection", "Select a point to inspect its evidence"); $("timeline-index").value = "0"; $("timeline-index").removeAttribute("aria-valuetext");
    renderChart(); renderTable(); renderInspector(); renderDeliveries();
    status("Open a JSONL recording or explore an example. You can also drop a file anywhere.");
  }
  $("open-file").addEventListener("click", () => $("evidence-file").click());
  $("evidence-file").addEventListener("change", (event) => { const file = event.target.files[0]; event.target.value = ""; void loadFile(file); });
  $("load-demo").addEventListener("click", () => {
    ++loadVersion; $("main").removeAttribute("aria-busy");
    try { accept(model.decode(window.VAPOR_EXAMPLE), "Example · memory pressure & recovery", true); }
    catch (error) { status(`Could not open example: ${error.message}`, true); }
  });
  $("clear").addEventListener("click", clear);
  $("stream").addEventListener("change", chooseStream);
  for (const button of document.querySelectorAll("[data-filter]")) button.addEventListener("click", () => { stateFilter = button.dataset.filter; applyFilter(); });
  $("search").addEventListener("input", applyFilter);
  $("evidence-rows").addEventListener("click", (event) => { const target = event.target.closest("tr[data-index]"); if (target) select(rows[Number(target.dataset.index)]); });
  $("previous").addEventListener("click", () => { const at = filtered.indexOf(selected); if (at > 0) select(filtered[at - 1], true); });
  $("next").addEventListener("click", () => { const at = filtered.indexOf(selected); if (at >= 0 && at + 1 < filtered.length) select(filtered[at + 1], true); });
  $("page-previous").addEventListener("click", () => { if (page > 0) { page -= 1; renderTable(); } });
  $("page-next").addEventListener("click", () => { if ((page + 1) * PAGE_SIZE < filtered.length) { page += 1; renderTable(); } });
  $("delivery-previous").addEventListener("click", () => { if (deliveryPage > 0) { deliveryPage -= 1; renderDeliveries(); } });
  $("delivery-next").addEventListener("click", () => { if (recording && (deliveryPage + 1) * DELIVERY_PAGE_SIZE < deliveryRows().length) { deliveryPage += 1; renderDeliveries(); } });
  $("timeline-index").addEventListener("input", (event) => chartSelect(Number(event.target.value)));
  $("timeline").addEventListener("click", (event) => {
    if (!rows.length) return;
    const point = $("timeline").createSVGPoint(); point.x = event.clientX; point.y = event.clientY;
    const local = point.matrixTransform($("timeline").getScreenCTM().inverse());
    chartSelect(Math.round((local.x - plot.left) / (plot.right - plot.left) * (rows.length - 1)));
  });
  $("export").addEventListener("click", () => {
    if (!recording || !filtered.length) return;
    try {
      const url = URL.createObjectURL(new Blob([model.exportRows(filtered, recording)], { type: "application/x-ndjson" }));
      const anchor = node("a"); anchor.href = url; anchor.download = example ? "vapor-example-selection.jsonl" : "vapor-evidence-selection.jsonl";
      document.body.append(anchor); anchor.click(); anchor.remove(); setTimeout(() => URL.revokeObjectURL(url), 1000);
      status(`Exported ${filtered.length} observations with their matching delivery records${example ? " from the example" : ""}.`);
    } catch (error) { status(`Export failed: ${error.message}`, true); }
  });
  for (const button of document.querySelectorAll(".guide-trigger")) button.addEventListener("click", () => $("guide").showModal());
  $("close-guide").addEventListener("click", () => $("guide").close());
  $("guide").addEventListener("click", (event) => { if (event.target === $("guide")) { const r = $("guide").getBoundingClientRect(); if (event.clientX < r.left || event.clientX > r.right || event.clientY < r.top || event.clientY > r.bottom) $("guide").close(); } });
  const links = [...document.querySelectorAll(".nav a")];
  function updateNavigation(id) {
    for (const link of links) { const active = link.hash === `#${id}`; link.classList.toggle("active", active); if (active) { link.setAttribute("aria-current", "location"); text("breadcrumb-section", link.getAttribute("aria-label")); } else link.removeAttribute("aria-current"); }
  }
  links.forEach((link) => link.addEventListener("click", () => updateNavigation(link.hash.slice(1))));
  if ("IntersectionObserver" in window) {
    const observer = new IntersectionObserver((entries) => { const visible = entries.filter((entry) => entry.isIntersecting); if (visible.length) updateNavigation(visible.at(-1).target.id); }, { rootMargin: "0px 0px -65% 0px", threshold: 0 });
    for (const link of links) observer.observe(document.querySelector(link.hash));
  }
  document.addEventListener("dragenter", (event) => { if ([...event.dataTransfer.types].includes("Files")) { event.preventDefault(); dragDepth += 1; $("drop-overlay").hidden = false; } });
  document.addEventListener("dragover", (event) => { if ([...event.dataTransfer.types].includes("Files")) { event.preventDefault(); event.dataTransfer.dropEffect = "copy"; } });
  document.addEventListener("dragleave", () => { dragDepth = Math.max(0, dragDepth - 1); if (!dragDepth) $("drop-overlay").hidden = true; });
  document.addEventListener("drop", (event) => { event.preventDefault(); dragDepth = 0; $("drop-overlay").hidden = true; const files = event.dataTransfer.files; if (files.length !== 1) status("Please open one recording at a time.", true); else void loadFile(files[0]); });
  window.addEventListener("blur", () => { dragDepth = 0; $("drop-overlay").hidden = true; });
  clear();
})();
