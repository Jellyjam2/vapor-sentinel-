# System Architecture

## Overview

Vapor Sentinel is a **real-time defensive system monitoring engine** built in Rust using a custom domain-specific language (DSL) for defining security actions. The system is designed for authorized monitoring scenarios, including validator node protection and automated incident response.

---

## Core Components

### 1. **VaporParser (Pest Grammar)**
- **File**: `src/vapor.pest`
- **Purpose**: Defines the syntax for security action declarations
- **Parser Type**: Pest parser combinator library
- **Grammar Rules**:
  ```
  vapor_func := "vapor_func" "{" statements "}"
  statements := (send_stmt | shred_stmt | if_stmt | while_stmt)*
  send_stmt := "send(" STRING ")"
  shred_stmt := "shred(" STRING ")"
  if_stmt := "if(" VARIABLE ") {" body "}"
  while_stmt := "while(" VARIABLE ") {" body "}"
  ```

### 2. **HardenedStore (State Vault)**
- **File**: `src/main.rs` (struct definition, line ~10)
- **Purpose**: Immutable, secure state container for monitoring metrics
- **Structure**:
  ```rust
  struct HardenedStore {
      vault: HashMap<String, u64>
  }
  ```
- **Key Operations**:
  - `new()` — Initialize empty vault
  - `refresh_global()` — Poll system metrics via sysinfo crate
  - `execute_body()` — Parse and execute DSL statements against current state

### 3. **Monitoring Loop**
- **File**: `src/main.rs` (main function, line ~74)
- **Polling Interval**: 4 seconds (tunable via `Duration::from_secs(4)`)
- **Cycle**:
  1. Refresh system metrics (total RAM, per-process consumption)
  2. Identify processes >50 MB
  3. Evaluate DSL actions against current state
  4. Execute send/shred/conditional actions
  5. Check for `EXIT` file signal
  6. Sleep before next cycle

### 4. **Security Action Executors**

#### Send Action (Webhook Alert)
- **Trigger**: `send("message")` statement
- **Behavior**:
  - Constructs JSON alert payload:
    ```json
    {
      "timestamp": "<ISO 8601>",
      "message": "<user message>",
      "ram_mb": <total system RAM>,
      "status": "VAPOR_SENTINEL_TRIGGERED"
    }
    ```
  - POSTs to configured webhook endpoint
  - Default endpoint: `https://webhook.site` (placeholder; override before production)

#### Shred Action (Secure File Deletion)
- **Trigger**: `shred("filepath")` statement
- **Behavior**:
  - Verifies file exists
  - Overwrites file content (implementation-dependent)
  - Deletes file metadata
  - No recovery possible after shred

#### Conditional Logic (if statement)
- **Trigger**: `if(VARIABLE) { body }` statement
- **Evaluation**: Checks if VARIABLE exceeds hardcoded threshold (>100 MB)
- **Body Execution**: Recursively evaluates nested statements if condition is true

#### Loop Logic (while statement)
- **Trigger**: `while(VARIABLE) { body }` statement
- **Behavior**: Repeats body execution while condition remains true

---

## Data Flow Diagram

```
┌─────────────────────┐
│   System Metrics    │
│   (sysinfo crate)   │
└──────────┬──────────┘
           │
           ├─ Total RAM
           ├─ Per-Process Memory
           └─ Process Names
           │
           ▼
┌──────────────────────┐
│  HardenedStore Vault │
│  (HashMap<K, u64>)   │
└──────────┬───────────┘
           │
           ├─ Stores metrics
           ├─ Refreshes every 4s
           └─ Triggers DSL evaluation
           │
           ▼
┌───────────────────────┐
│  DSL Parser & Executor│
│  (Pest grammar)       │
└──────────┬────────────┘
           │
           ├─ Parse vapor_func code
           ├─ Evaluate conditionals
           └─ Execute actions
           │
           ├────────────────────┬──────────────────┬──────────────────┐
           │                    │                  │                  │
           ▼                    ▼                  ▼                  ▼
      ┌────────┐          ┌─────────┐        ┌──────────┐      ┌──────────┐
      │  SEND  │          │  SHRED  │        │ IF Eval  │      │ WHILE    │
      │ Webhook│          │ Delete  │        │ Recurse  │      │ Loop     │
      │ Alert  │          │ File    │        │          │      │          │
      └────────┘          └─────────┘        └──────────┘      └──────────┘
```

---

## Key Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `pest` | 2.7 | Parser combinator library |
| `pest_derive` | 2.7 | Procedural macros for grammar |
| `sysinfo` | 0.29.11 | System metrics collection |
| `ureq` | 2.12.1 | HTTP requests for webhooks |
| `zeroize` | 1.7 | Secure memory cleanup |
| `region` | 3.0 | Memory protection APIs |
| `serde_json` | 1.0 | JSON serialization |
| `chrono` | 0.4 | Timestamp generation |
| `wasmtime` | 16.0 | WASM runtime (future) |

---

## Security Guarantees

### Memory Safety
- **Zeroization**: All sensitive data (vault, intermediate computations) are overwritten before exit
- **DROP Implementation**: `impl Drop for HardenedStore` ensures memory is explicitly cleared
- **No Buffer Overflows**: Rust type system prevents classic memory vulnerabilities

### Explicit Approval
- No persistence mechanisms—monitoring halts immediately on process exit
- `EXIT` file provides user-controllable termination signal
- No background daemons or service installation

### Transparent Operation
- All actions logged to stdout (configurable to stderr or files in future versions)
- Webhook payloads visible for audit/review before sending
- No hidden network connections or covert data exfiltration

### Bounded Destruction
- Shred action targets explicit file paths only—no wildcards or recursive deletion
- User provides exact filepath; no guessing or auto-expansion
- Verification step before file deletion

---

## Execution Flow (Pseudocode)

```rust
fn main() {
    // 1. Initialize monitoring
    let mut store = HardenedStore::new();
    let mut sys = System::new_all();
    
    // 2. Hardcoded DSL code (to be parameterized in v0.2.0)
    let code = r#"
        vapor_func {
            send("System temperature alert")
            if(SYSTEM_RAM) {
                shred("bounty_hunt.log")
            }
        }
    "#;
    
    // 3. Parse DSL
    let parse = VaporParser::parse(Rule::vapor_func, code)?;
    
    // 4. Main loop
    loop {
        // 4a. Refresh system state
        store.refresh_global(&mut sys);
        
        // 4b. Execute DSL body
        let body = /* extract from parsed tree */;
        store.execute_body(&body, &sys)?;
        
        // 4c. Check for exit signal
        if Path::new("EXIT").exists() {
            break;
        }
        
        // 4d. Sleep before next cycle
        thread::sleep(Duration::from_secs(4));
    }
    
    // 5. On exit, DROP impl zeroizes vault
    drop(store);
}
```

---

## Future Enhancements (v0.2.0+)

### Configuration System
- TOML-based config file for DSL code, webhook URLs, thresholds
- Separate monitoring rules from binary

### Wasmtime Sandbox
- Execute untrusted plugin code in isolated WASM environment
- Restrict system access within plugin VM

### Multi-Destination Alerting
- Support for email, Slack, Discord webhooks
- Conditional alert routing based on severity

### Hardware Protection
- Leverage `region` crate for page-level memory protection
- Prevent unauthorized access to vault contents

### Enhanced Filtering
- Process name pattern matching
- PID-based targeting
- User/group-based monitoring restrictions

---

## Deployment Checklist

Before deploying Vapor Sentinel, ensure:
- [ ] SECURITY.md reviewed and understood
- [ ] Written authorization obtained from system owner
- [ ] Webhook endpoint configured and tested
- [ ] Alert retention policy established
- [ ] Exit procedure documented and accessible
- [ ] Audit logging enabled
- [ ] Incident response plan in place

For questions or clarifications, refer to `SECURITY.md` and contact the maintainers.
