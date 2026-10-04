const sampleEvidence={sequence:1842,metric:"SYSTEM_USED_MEMORY_MB",value:118,threshold:100,delta:12,state:"ANOMALOUS",reason:"Threshold exceeded"};
document.querySelector("#sequence").textContent=sampleEvidence.sequence;
document.querySelector("#metric").textContent=sampleEvidence.metric;
document.querySelector("#value").textContent=sampleEvidence.value;
document.querySelector("#threshold").textContent=sampleEvidence.threshold+" MiB";
document.querySelector("#state").textContent=sampleEvidence.state;
document.querySelector("#reason").textContent=sampleEvidence.reason;
document.querySelector(".delta").textContent="+"+sampleEvidence.delta+" MiB";
// Presentation-only sample data. Live transport is intentionally not implied.