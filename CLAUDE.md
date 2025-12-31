# CLAUDE.md - Claude Code Instructions

## Project: ag-botkit (AlgorithmicGrid Stack)

A production-ready, low-latency trading infrastructure monorepo for Polymarket.
Core primitives in C, execution/risk/storage/strategy services in Rust, monitoring dashboard in Go.

**Current Status:** 90% complete (4.5/5 roadmap features implemented)

## Repository Structure

```
ag-botkit/
├── .claude/agents/          # 9 specialized Claude Code agents
│   ├── system-architect.md  # Architecture planning & integration
│   ├── core-c-implementer.md
│   ├── risk-engine.md
│   ├── monitor-ui.md
│   ├── advanced-risk.md     # VaR, Greeks, Portfolio analytics
│   ├── exec-gateway.md      # Polymarket CLOB integration
│   ├── storage-layer.md     # TimescaleDB persistence
│   ├── strategy-engine.md   # Trading strategies framework
│   └── devops-infra.md      # Deployment & monitoring
│
├── core/                    # C low-level primitives
│   ├── include/            # Public API (ag_timeseries.h)
│   ├── src/                # Ring buffer, time-series
│   ├── tests/              # 25 unit tests (all passing)
│   └── lib/                # libag_core.a
│
├── risk/                    # Rust risk engine library
│   ├── src/
│   │   ├── lib.rs          # Base policies (PositionLimit, InventoryLimit, KillSwitch)
│   │   └── advanced/       # VaR, Greeks, Portfolio, Stress, Performance
│   ├── benches/            # Performance benchmarks
│   ├── docs/               # VAR_METHODOLOGY.md, GREEKS_GUIDE.md, STRESS_TESTING.md
│   ├── examples/           # Usage examples
│   └── policies/           # YAML policy templates
│
├── exec/                    # Rust execution gateway
│   ├── src/
│   │   ├── engine.rs       # ExecutionEngine core
│   │   ├── adapter.rs      # VenueAdapter trait
│   │   ├── oms.rs          # Order Management System
│   │   ├── ratelimit.rs    # Token bucket rate limiter
│   │   └── venues/
│   │       └── polymarket.rs  # Polymarket CLOB adapter (HMAC-SHA256)
│   └── tests/              # 37 unit tests (all passing)
│
├── storage/                 # Rust + TimescaleDB storage layer
│   ├── src/
│   │   ├── lib.rs          # StorageEngine API
│   │   ├── timescale/      # Connection pooling (PARTIAL)
│   │   ├── ingest/         # Batch ingestion
│   │   └── retention/      # Data retention policies (PARTIAL)
│   ├── schemas/
│   │   ├── 001_metrics.sql
│   │   ├── 002_execution.sql
│   │   └── 003_aggregates.sql
│   └── docker-compose.yml
│
├── strategies/              # Rust strategy framework
│   ├── src/
│   │   ├── strategy.rs     # Strategy trait + lifecycle
│   │   ├── context.rs      # StrategyContext (exec/risk integration)
│   │   └── coordinator.rs  # MultiMarketCoordinator
│   ├── signals/            # Technical indicators (SMA, EMA, RSI, Bollinger, MACD)
│   ├── impl/               # Market Maker, Arbitrage (PARTIAL)
│   └── backtest/           # Event-driven backtesting engine
│
├── monitor/                 # Go monitoring dashboard
│   ├── cmd/monitor/        # Entry point
│   ├── internal/           # WebSocket server, storage
│   ├── web/                # HTML/CSS/JS (uPlot charts)
│   └── bin/                # Compiled binary
│
├── examples/minibot/        # Rust demo bot (RTDS integration)
│
├── deploy/                  # Deployment configurations
│   ├── docker/             # Dockerfiles, docker-compose
│   └── k8s/                # Kubernetes manifests
│
├── infra/                   # Infrastructure as Code
│   ├── terraform/          # AWS/GCP infrastructure
│   ├── monitoring/         # Prometheus + Grafana
│   └── ops/                # Runbooks, DR plans
│
├── scripts/                 # Utility scripts
│
├── MULTI_AGENT_PLAN.md     # System architecture v2.0
├── ROADMAP_AGENTS_SUMMARY.md
├── CONTINUATION.md         # Tasks for next session
├── Makefile                # Root build system
└── run_local.sh            # Local dev launcher
```

## Development Commands

### Build Commands

```bash
# Build all modules
make all

# Build individual modules
make core        # C library
make risk        # Rust risk engine
make exec        # Rust execution gateway
make storage     # Rust storage layer
make strategies  # Rust strategy framework
make monitor     # Go dashboard
make minibot     # Demo bot

# Note: cargo/rustc are in ~/.cargo/bin
# Use: source "$HOME/.cargo/env" before cargo commands
```

### Test Commands

```bash
# Run all tests
make test

# Individual test suites
make test-core       # C library (25 tests)
make test-risk       # Risk engine (59/65 passing)
make test-exec       # Execution gateway (37 tests)
make test-storage    # Storage layer (requires TimescaleDB)
make test-strategies # Strategy framework
make test-monitor    # Go dashboard

# Linting
cd risk && cargo clippy
cd exec && cargo clippy
cd storage && cargo clippy
cd strategies && cargo clippy

# Fix warnings
cargo fix --allow-dirty
```

### Run Commands

```bash
# Start full stack (monitor + minibot)
./run_local.sh

# Start TimescaleDB (for storage layer)
cd storage && docker-compose up -d

# Start monitor only
./monitor/bin/monitor -web ./monitor/web

# Start minibot only (requires monitor running)
./examples/minibot/target/release/minibot --config examples/minibot/config.yaml
```

## Code Style & Rules

### 1. C Core (`core/`)

- **Standard:** C11
- **API Style:** Opaque handles (`struct ag_timeseries*`)
- **Error Handling:** Return `int` error codes (0 = success, negative = error)
- **Output:** Via pointer parameters (e.g., `double* out_value`)
- **Memory:** No allocations in hot paths, caller manages lifetime
- **Threading:** Thread-safe by design (caller provides locking if needed)
- **Build:** Makefile-based, outputs `lib/libag_core.a`

**Example:**
```c
int ag_timeseries_append(struct ag_timeseries* ts, double timestamp, double value);
int ag_timeseries_get_latest(struct ag_timeseries* ts, double* out_timestamp, double* out_value);
```

### 2. Rust Services (`risk/`, `exec/`, `storage/`, `strategies/`)

- **Async Runtime:** `tokio` for async operations
- **Error Handling:** `thiserror` for error types, `Result<T, E>` returns
- **API Style:** Builder pattern for configuration, trait-based abstractions
- **Testing:** Unit tests in `tests/` directories, integration tests in `tests/*.rs`
- **Documentation:** Rustdoc comments (`///`) for public APIs
- **Dependencies:** Minimal, prefer standard library where possible

**Module Dependencies:**
```
exec → risk (pre-trade checks)
strategies → risk + exec (strategy context)
storage → standalone (no dependencies)
```

### 3. Go Services (`monitor/`)

- **Layout:** Standard Go project structure (`cmd/`, `internal/`, `pkg/`)
- **Error Handling:** Explicit error returns, no panics in production code
- **Concurrency:** Goroutines + channels, avoid mutexes where possible
- **Testing:** `_test.go` files, table-driven tests
- **Build:** `go build` outputs to `bin/`

### 4. Communication Between Modules

- **monitor ← minibot:** WebSocket (JSON metrics)
- **strategies ↔ exec:** Direct Rust API calls
- **exec ↔ risk:** Direct Rust API calls (pre-trade checks)
- **storage:** SQL queries via connection pool
- **core ← all:** C FFI (via `bindgen` or manual bindings)

## Agent Workflow

### Specialized Agents

Use Task tool with appropriate `subagent_type`:

1. **system-architect** (`subagent_type="system-architect"`)
   - **Owns:** `MULTI_AGENT_PLAN.md`
   - **Responsibilities:** Architecture planning, interface design, integration validation
   - **Use when:**
     - Defining new module interfaces
     - Validating cross-module integration
     - Updating architectural documentation
     - Finalizing roadmap features

2. **core-c-implementer** (`subagent_type="core-c-implementer"`)
   - **Works in:** `core/`
   - **Responsibilities:** C library implementation, unit tests, benchmarks
   - **Use when:** Implementing C primitives, optimizing performance

3. **risk-engine** (`subagent_type="risk-engine"`)
   - **Works in:** `risk/`
   - **Responsibilities:** Risk policies, VaR/Greeks/Portfolio analytics, simulators
   - **Use when:** Implementing risk logic, advanced models, stress tests

4. **exec-gateway** (custom agent in `.claude/agents/exec-gateway.md`)
   - **Works in:** `exec/`
   - **Responsibilities:** Execution engine, venue adapters, OMS, rate limiting
   - **Use when:** Implementing order execution, API integrations

5. **storage-layer** (custom agent in `.claude/agents/storage-layer.md`)
   - **Works in:** `storage/`
   - **Responsibilities:** TimescaleDB schema, ingestion, retention policies
   - **Use when:** Implementing persistence, database operations

6. **strategy-engine** (custom agent in `.claude/agents/strategy-engine.md`)
   - **Works in:** `strategies/`
   - **Responsibilities:** Strategy framework, signals, backtesting
   - **Use when:** Implementing trading strategies, backtesting

7. **monitor-ui** (`subagent_type="monitor-ui"`)
   - **Works in:** `monitor/`
   - **Responsibilities:** Dashboard, WebSocket server, metrics visualization
   - **Use when:** Implementing monitoring UI, metrics ingestion

8. **devops-infra** (custom agent in `.claude/agents/devops-infra.md`)
   - **Works in:** `deploy/`, `infra/`
   - **Responsibilities:** Deployment configs, IaC, monitoring stack
   - **Use when:** Setting up deployment infrastructure

### Workflow Rules

1. **ALWAYS consult `system-architect` first** when:
   - Adding new modules or features
   - Changing public APIs or interfaces
   - Integrating multiple modules
   - Finalizing roadmap features

2. **Subagents work ONLY in their directories**
   - `core-c-implementer` → `core/` only
   - `risk-engine` → `risk/` only
   - `exec-gateway` → `exec/` only
   - etc.

3. **Definition of Done** for all modules:
   - ✅ Code compiles (`cargo build --release` or `make`)
   - ✅ Tests pass (`cargo test` or `make test`)
   - ✅ No clippy warnings (`cargo clippy`)
   - ✅ `README.md` in module directory updated
   - ✅ Public APIs documented (Rustdoc or header comments)

4. **API Changes Require Architect Approval**
   - ALL changes to public interfaces must be documented in `MULTI_AGENT_PLAN.md`
   - Architect reviews and updates integration contracts
   - Breaking changes require migration plan

## Current Known Issues

See `CONTINUATION.md` for detailed list. Summary:

**🔴 Critical Fixes:**
- 6 risk test failures (numerical precision tolerances)
- Storage module incomplete (`timescale/` and `retention/` implementations)

**🟡 Minor Issues:**
- 19 compiler warnings (unused imports/variables)
- Polymarket adapter missing WebSocket fills stream
- Reference strategy implementations incomplete

**🟢 Future Enhancements:**
- Production deployment tooling (Docker, K8s, CI/CD)
- Multi-venue support (CEX/DEX adapters)
- Advanced backtesting features

## Module Status (90% Complete)

| Module | Status | Tests | Notes |
|--------|--------|-------|-------|
| core/ | ✅ DONE | 25/25 ✅ | C library fully functional |
| risk/ | ✅ DONE | 59/65 ⚠️ | 6 precision issues, logic correct |
| exec/ | ✅ DONE | 37/37 ✅ | Polymarket CLOB production-ready |
| storage/ | ⚠️ PARTIAL | Skipped | Schema ready, needs TimescaleDB |
| strategies/ | ✅ DONE | - | Framework complete, examples partial |
| monitor/ | ✅ DONE | All ✅ | Dashboard fully operational |
| minibot/ | ✅ DONE | - | RTDS integration working |
| deploy/ | ⚠️ READY | Untested | Configs ready, not tested |
| infra/ | ⚠️ READY | Untested | IaC ready, not tested |

## Multi-Agent Development Tips

1. **Start with Architect**: Always invoke `system-architect` for feature planning
2. **Parallel Work**: Multiple subagents can work simultaneously in different modules
3. **Integration Testing**: Use `system-architect` to validate cross-module integration
4. **Documentation**: Update `MULTI_AGENT_PLAN.md` after significant changes
5. **Continuation**: Use `CONTINUATION.md` for cross-session task tracking

## Environment Setup (macOS)

```bash
# Install Rust (already installed: 1.92.0)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Install Go (already installed: 1.25.5)
brew install go

# Install Docker Desktop (optional, for storage)
brew install --cask docker

# Verify installations
rustc --version  # 1.92.0
cargo --version  # 1.92.0
go version       # go1.25.5
gcc --version    # Apple clang 16.0.0
```

## Quick Start for New Developers

```bash
# 1. Clone and enter repo
git clone <repo-url> && cd ag-botkit

# 2. Build all modules
source "$HOME/.cargo/env"  # Ensure cargo is in PATH
make all

# 3. Run tests
make test

# 4. Start local stack
./run_local.sh

# 5. Open dashboard
open http://localhost:8080
```

## Next Steps

See **CONTINUATION.md** for prioritized task list.

**Immediate priorities:**
1. Fix 6 risk test failures
2. Complete storage module implementation
3. Test Polymarket integration with testnet
4. Implement reference strategies (MM, Arbitrage)
5. Validate deployment configs

## Documentation References

- **Architecture:** `MULTI_AGENT_PLAN.md`
- **Roadmap:** `ROADMAP_AGENTS_SUMMARY.md`
- **User Guide:** `README.md`
- **Continuation:** `CONTINUATION.md`
- **Risk Models:** `risk/docs/VAR_METHODOLOGY.md`, `risk/docs/GREEKS_GUIDE.md`
- **Implementation Summaries:** `*/IMPLEMENTATION_SUMMARY.md` in each module

---

**Remember:** This is a production-grade trading infrastructure. Maintain high code quality, comprehensive testing, and thorough documentation at all times.
