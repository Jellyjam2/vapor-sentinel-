use pest::Parser;
use pest_derive::Parser;
use serde_json::json;
use std::{collections::HashMap, fs, thread, time::Duration};
use sysinfo::{ProcessExt, System, SystemExt}; // NEW: Bulletproof JSON

#[derive(Parser)]
#[grammar = "vapor.pest"]
pub struct VaporParser;

struct HardenedStore {
    vault: HashMap<String, u64>,
}

impl HardenedStore {
    fn new() -> Self {
        Self {
            vault: HashMap::new(),
        }
    }

    fn refresh_global(&mut self, sys: &mut System) {
        sys.refresh_all();
        let total_ram = sys.used_memory() / 1024 / 1024;
        self.vault.insert("SYSTEM_RAM".to_string(), total_ram);

        println!("--- 🛰️ GLOBAL RADAR: System {}MB ---", total_ram);
        for (pid, process) in sys.processes() {
            let mb = process.memory() / 1024 / 1024;
            if mb > 50 {
                println!("🔎 ACTIVE: {} ({}MB) [PID: {}]", process.name(), mb, pid);
            }
        }
    }

    fn execute_body(&mut self, pairs: Vec<pest::iterators::Pair<Rule>>) {
        for pair in pairs {
            match pair.as_rule() {
                Rule::send_stmt => {
                    if let Some(inner) = pair.clone().into_inner().next() {
                        let msg = inner.as_str();
                        // 2150 SURGICAL JSON: Manual construction to avoid crate bugs
                        let alert_data = json!({
                            "alert": msg,
                            "ram_mb": self.vault.get("SYSTEM_RAM").unwrap_or(&0),
                            "status": "VAPOR_SENTINEL_TRIGGERED"
                        });

                        // Replace with your Webhook.site URL
                        let _ = ureq::post("https://webhook.site")
                            .set("Content-Type", "application/json")
                            .send_string(&alert_data.to_string());

                        println!("--- 📡 SIGNAL BURST SENT: {} ---", msg);
                    }
                }
                Rule::shred_stmt => {
                    if let Some(inner) = pair.into_inner().next() {
                        let path = inner.as_str();
                        let _ = fs::remove_file(path);
                        println!("--- 🔥 VAPORIZED: {} SHREDDED ---", path);
                    }
                }
                Rule::if_stmt => {
                    let mut inner = pair.into_inner();
                    if let Some(cond_token) = inner.next() {
                        let cond = cond_token.as_str().trim();
                        let body_pair = inner.next().unwrap();
                        if *self.vault.get(cond).unwrap_or(&0) > 100 {
                            println!("--- 🎯 TRIGGER: {} exceeds 100MB! ---", cond);
                            self.execute_body(body_pair.into_inner().collect());
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

impl Drop for HardenedStore {
    fn drop(&mut self) {
        println!("--- 2150 SECURITY: SHREDDING VAULT ---");
        self.vault.clear();
        println!("--- 2150 SECURITY: SYSTEM CLEAN. ---");
    }
}

fn main() -> anyhow::Result<()> {
    let mut sys = System::new_all();
    let mut engine = HardenedStore::new();

    let code = "vapor sentinel() { 
        if(SYSTEM_RAM) { 
            send(\"CRITICAL_RAM_DETECTED\"); 
            shred(\"tests/bounty_hunt.log\"); 
        } 
    }";

    let parse = VaporParser::parse(Rule::vapor_func, code)?.next().unwrap();
    let body: Vec<_> = parse
        .into_inner()
        .find(|p| p.as_rule() == Rule::body)
        .unwrap()
        .into_inner()
        .collect();

    println!("--- 2150 SENTINEL ACTIVE (Create 'EXIT' file to dissolve) ---");

    loop {
        engine.refresh_global(&mut sys);
        engine.execute_body(body.clone());

        if std::path::Path::new("EXIT").exists() {
            let _ = fs::remove_file("EXIT");
            break;
        }
        thread::sleep(Duration::from_secs(4));
    }

    drop(engine);
    Ok(())
}
