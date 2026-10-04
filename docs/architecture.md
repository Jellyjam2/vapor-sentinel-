# System Architecture

## Product Ownership

**Vapor Sentinel is a product of Titan Black Swan TECHNOLOGIES.**

## Overview

Vapor Sentinel is a **real-time defensive system monitoring engine** built in Rust using a custom domain-specific language (DSL) for defining security actions. The system is designed for authorized monitoring scenarios, including validator node protection and automated incident response.

---

## Core Components

### 1. **VaporParser (Pest Grammar)**
- **File**: `src/vapor.pest`
- **Purpose**: Defines the syntax for security action declarations
- **Parser Type**: Pest parser combinator library
