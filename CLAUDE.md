# CLAUDE.md

## Project: AlgorithmicGrid Stack (ag-stack)
A low-latency, modular trading infrastructure monorepo for CEX/DEX/Polymarket.
Core logic in C, execution/risk/monitoring services in Rust/Go.

## Repository Structure (Target)
- `core/`    : C low-level primitives (orderbook, ring buffers, fast math). No allocations in hot paths.
- `exec/`    : Execution gateway (Rust). Order management, rate limits, venue adapters.
- `risk/`    : Risk engine library (Rust). Policy evaluation, pre-trade checks, inventory limits.
- `monitor/` : Local monitoring dashboard (Go/TS). Metrics ingestion -> uPlot charts.
- `examples/`: End-to-end bot examples (e.g., `minibot`).
- `.claude/` : Agent definitions and project-specific instructions.

## Development Commands
- **Build All**: `./scripts/build_all.sh` (agent must create this)
- **Test Core**: `cd core && make test`
- **Test Exec**: `cd exec && cargo test`
- **Test Risk**: `cd risk && cargo test`
- **Run Monitor**: `cd monitor && go run cmd/server/main.go`
- **End-to-End**: `./run_local.sh` (starts monitor + mock bot)

## Code Style & Rules
1. **C Core**:
   - C11 standard. Opaque handles (`struct ag_ob*`).
   - Return `int` error codes, output via pointers.
   - No global state. Thread-safe by design (caller locks).

2. **Rust/Go Services**:
   - Rust: `tokio` for async, `thiserror` for errors.
   - Go: Standard layout (`cmd/`, `internal/`, `pkg/`).
   - Communications via defined interfaces (gRPC or zero-copy IPC, TBD by Architect).

3. **Agent Workflow**:
   - **Architect** owns `MULTI_AGENT_PLAN.md` and defines interfaces.
   - Subagents (`core-c`, `risk-engine`, `monitor-ui`) work ONLY in their directories.
   - ALL changes to public APIs must be approved in the Plan first.
   - "Definition of Done": Code compiles, tests pass, and `README.md` in the subfolder is updated.

## Subagents
Use `/agents` to invoke specialized roles:
- `architect`: Plan & Integrations.
- `core-c`: C implementation.
- `risk-engine`: Risk logic.
- `monitor-ui`: Dashboard & Metrics.
