# Roadmap Agent Architecture Summary

**Date:** 2025-12-31
**Architect:** system-architect agent
**Version:** 2.0

## Overview

This document summarizes the specialized sub-agent architecture designed for implementing the ag-botkit production roadmap. All agents are now defined with clear boundaries, interface contracts, and integration points.

## Roadmap Features → Agent Mapping

| Feature | Agent | Directory | Agent Definition File |
|---------|-------|-----------|----------------------|
| CLOB API Integration | `exec-gateway` | `exec/` | `.claude/agents/exec-gateway.md` |
| Persistent Storage | `storage-layer` | `storage/` | `.claude/agents/storage-layer.md` |
| Advanced Risk Models | `advanced-risk` | `risk/src/advanced/` | `.claude/agents/advanced-risk.md` |
| Multi-Market Strategies | `strategy-engine` | `strategies/` | `.claude/agents/strategy-engine.md` |
| Production Deployment | `devops-infra` | `deploy/`, `infra/` | `.claude/agents/devops-infra.md` |

## All Agents (MVP + Roadmap)

### MVP Agents (Existing)

1. **system-architect**
   - **File:** `.claude/agents/system-architect.md`
   - **Role:** Architectural orchestrator, MULTI_AGENT_PLAN.md owner
   - **Directory:** All (read-only except MULTI_AGENT_PLAN.md)
   - **Size:** 75 lines

2. **core-c-implementer**
   - **File:** `.claude/agents/core-c-implementer.md`
   - **Role:** C library implementation for low-level primitives
   - **Directory:** `core/`
   - **Size:** 63 lines

3. **risk-engine**
   - **File:** `.claude/agents/risk-engine.md`
   - **Role:** Policy-based risk evaluation and simulation
   - **Directory:** `risk/`
   - **Size:** 88 lines

4. **monitor-ui**
   - **File:** `.claude/agents/monitor-ui.md`
   - **Role:** Real-time monitoring dashboard and WebSocket server
   - **Directory:** `monitor/`
   - **Size:** 63 lines

### Roadmap Agents (New)

5. **exec-gateway**
   - **File:** `.claude/agents/exec-gateway.md`
   - **Role:** Order execution, exchange connectivity, API integration
   - **Directory:** `exec/`
   - **Size:** 193 lines
   - **Key Responsibilities:**
     - Polymarket CLOB API adapter
     - Order management system (OMS)
     - Rate limiting and throttling
     - Venue adapter interface
     - Pre-trade risk integration

6. **storage-layer**
   - **File:** `.claude/agents/storage-layer.md`
   - **Role:** Persistent storage, TimescaleDB integration, data retention
   - **Directory:** `storage/`
   - **Size:** 383 lines
   - **Key Responsibilities:**
     - TimescaleDB hypertable schemas
     - High-throughput metric ingestion
     - Execution history storage
     - Query API for analytics
     - Data retention and compression

7. **advanced-risk**
   - **File:** `.claude/agents/advanced-risk.md`
   - **Role:** Quantitative risk models, VaR, Greeks, portfolio analytics
   - **Directory:** `risk/src/advanced/`
   - **Size:** 426 lines
   - **Key Responsibilities:**
     - VaR models (Historical, Parametric, Monte Carlo)
     - Black-Scholes Greeks calculation
     - Portfolio risk analytics
     - Stress testing framework
     - Performance metrics (Sharpe, Sortino)

8. **strategy-engine**
   - **File:** `.claude/agents/strategy-engine.md`
   - **Role:** Trading strategy framework, multi-market coordination
   - **Directory:** `strategies/`
   - **Size:** 532 lines
   - **Key Responsibilities:**
     - Strategy trait and lifecycle management
     - Multi-market coordinator
     - Signal generation framework
     - Reference strategies (market making, arbitrage)
     - Backtesting engine

9. **devops-infra**
   - **File:** `.claude/agents/devops-infra.md`
   - **Role:** Deployment automation, infrastructure as code, production operations
   - **Directory:** `deploy/`, `infra/`
   - **Size:** 710 lines
   - **Key Responsibilities:**
     - Docker containerization
     - Kubernetes orchestration
     - CI/CD pipelines (GitHub Actions)
     - Terraform infrastructure
     - Monitoring stack (Prometheus/Grafana)

## Implementation Phases

### Phase 1: Storage Layer (Weeks 1-2)
**Agent:** `storage-layer`
**Dependencies:** None
**Deliverables:**
- TimescaleDB docker-compose setup
- Metrics and execution hypertable schemas
- Batch ingestion pipeline (>10k/sec)
- Query API with time-range filters
- Compression and retention policies
- Migration system

### Phase 2: Execution Gateway (Weeks 3-4)
**Agent:** `exec-gateway`
**Dependencies:** `risk-engine` (existing), `monitor-ui` (existing)
**Deliverables:**
- ExecutionEngine API and VenueAdapter trait
- Polymarket CLOB adapter
- Order state tracking
- Pre-trade risk integration
- Rate limiting per venue
- Execution metrics emission

### Phase 3: Advanced Risk (Weeks 5-6)
**Agent:** `advanced-risk`
**Dependencies:** `risk-engine` (extends)
**Deliverables:**
- Historical, Parametric, Monte Carlo VaR
- Black-Scholes Greeks calculation
- Portfolio volatility and correlation
- Stress testing scenarios
- VaR backtesting framework
- Integration with base risk policies

### Phase 4: Strategy Engine (Weeks 7-8)
**Agent:** `strategy-engine`
**Dependencies:** `exec-gateway`, `advanced-risk`, `risk-engine`
**Deliverables:**
- Strategy trait with lifecycle hooks
- MultiMarketCoordinator
- Market making strategy
- Cross-market arbitrage strategy
- Signal generation framework
- Backtesting engine

### Phase 5: Production Deployment (Weeks 9-10)
**Agent:** `devops-infra`
**Dependencies:** All modules
**Deliverables:**
- Docker images for all components
- Kubernetes manifests with HPA
- CI/CD pipeline (GitHub Actions)
- Terraform infrastructure provisioning
- Prometheus/Grafana monitoring
- Deployment runbooks

## Agent Coordination Matrix

| Agent | Reads From | Writes To | Coordinates With |
|-------|------------|-----------|------------------|
| system-architect | All | MULTI_AGENT_PLAN.md, scripts/ | All agents |
| core-c-implementer | core/ | core/ | system-architect |
| risk-engine | risk/ | risk/ | system-architect |
| advanced-risk | risk/ | risk/src/advanced/ | risk-engine, system-architect |
| monitor-ui | monitor/ | monitor/ | system-architect |
| exec-gateway | exec/, risk/, monitor/ | exec/ | system-architect, risk-engine |
| storage-layer | storage/ | storage/ | system-architect |
| strategy-engine | strategies/, exec/, risk/ | strategies/ | system-architect, exec-gateway |
| devops-infra | All | deploy/, infra/ | system-architect, all agents |

## Interface Contracts Summary

### exec-gateway ↔ risk-engine
```rust
// Pre-trade risk check
let risk_decision = risk_engine.evaluate(&RiskContext {
    market_id: order.market.clone(),
    current_position: get_position(&order.market),
    proposed_size: order.size,
    inventory_value_usd: get_total_inventory(),
}).await?;
```

### exec-gateway ↔ monitor-ui
```json
// Execution metrics
{
  "timestamp": 1735689600000,
  "metric_type": "gauge",
  "metric_name": "exec.latency_ms",
  "value": 45.2,
  "labels": {"venue": "polymarket"}
}
```

### storage-layer ↔ monitor-ui
```rust
// Metric persistence
storage_engine.insert_metrics_batch(metrics).await?;
```

### storage-layer ↔ exec-gateway
```rust
// Execution history storage
execution_store.store_order(order).await?;
execution_store.store_fill(fill).await?;
```

### strategy-engine ↔ exec-gateway
```rust
// Order submission
let order_id = ctx.exec_engine.lock().await.submit_order(order).await?;
```

### strategy-engine ↔ risk-engine
```rust
// Strategy-level risk checks
let risk_decision = ctx.risk_engine.lock().await.evaluate(&risk_ctx).await?;
```

### advanced-risk ↔ risk-engine
```rust
// Extended policy rules
pub enum PolicyRule {
    // Existing
    PositionLimit { ... },
    // Advanced
    VarLimit { max_var_usd: f64, confidence_level: f64 },
    GreeksLimit { max_delta: f64, max_gamma: f64 },
}
```

## Quality Gates

**All Agents Must Meet:**

1. **Code Quality**
   - Zero compiler warnings
   - Rust: `cargo clippy` clean
   - C: `-Wall -Wextra -Wpedantic` clean
   - Go: `go vet` clean

2. **Testing**
   - Unit test coverage >80%
   - Integration tests for all critical paths
   - Performance tests meet targets

3. **Documentation**
   - README.md in module directory
   - Public API documentation
   - Integration examples
   - Usage instructions

4. **Security**
   - No hardcoded secrets
   - External configuration
   - Input validation
   - Error handling

## Directory Structure After Roadmap

```
ag-botkit/
├── .claude/
│   └── agents/
│       ├── system-architect.md        (MVP)
│       ├── core-c-implementer.md      (MVP)
│       ├── risk-engine.md             (MVP)
│       ├── monitor-ui.md              (MVP)
│       ├── exec-gateway.md            (NEW)
│       ├── storage-layer.md           (NEW)
│       ├── advanced-risk.md           (NEW)
│       ├── strategy-engine.md         (NEW)
│       └── devops-infra.md            (NEW)
├── core/                              (MVP - C library)
├── risk/                              (MVP - Rust library)
│   └── src/advanced/                  (NEW - Advanced risk models)
├── monitor/                           (MVP - Go dashboard)
├── examples/minibot/                  (MVP - Demo bot)
├── exec/                              (NEW - Execution gateway)
├── storage/                           (NEW - TimescaleDB layer)
├── strategies/                        (NEW - Strategy framework)
├── deploy/                            (NEW - Containerization)
│   ├── docker/
│   └── k8s/
├── infra/                             (NEW - Infrastructure)
│   ├── terraform/
│   ├── monitoring/
│   └── ops/
├── MULTI_AGENT_PLAN.md               (UPDATED v2.0)
├── ROADMAP_AGENTS_SUMMARY.md         (THIS FILE)
└── README.md

Total Agents: 9 (4 MVP + 5 Roadmap)
```

## Usage Guidelines

### Invoking Agents

**For CLOB API work:**
```
Use the exec-gateway agent to implement order placement on Polymarket CLOB
```

**For database persistence:**
```
Use the storage-layer agent to store metrics in TimescaleDB
```

**For advanced risk analytics:**
```
Use the advanced-risk agent to implement VaR calculation
```

**For trading strategies:**
```
Use the strategy-engine agent to build a market making strategy
```

**For production deployment:**
```
Use the devops-infra agent to containerize and deploy to Kubernetes
```

### Architect Workflow

When implementing a roadmap feature:

1. **Review MULTI_AGENT_PLAN.md** - Understand defined contracts
2. **Assign to appropriate agent** - Use agent matching feature domain
3. **Monitor implementation** - Ensure agent stays within boundaries
4. **Validate integration** - Test contracts between modules
5. **Update plan** - Document any architectural changes

## Next Steps

1. **Week 1-2:** Invoke `storage-layer` agent to implement TimescaleDB integration
2. **Week 3-4:** Invoke `exec-gateway` agent to build Polymarket CLOB adapter
3. **Week 5-6:** Invoke `advanced-risk` agent to add VaR and Greeks models
4. **Week 7-8:** Invoke `strategy-engine` agent to create strategy framework
5. **Week 9-10:** Invoke `devops-infra` agent to deploy to production

## References

- **Architecture Plan:** `/Users/yaroslav/ag-botkit/MULTI_AGENT_PLAN.md`
- **Agent Definitions:** `/Users/yaroslav/ag-botkit/.claude/agents/`
- **Project Instructions:** `/Users/yaroslav/ag-botkit/CLAUDE.md`

---

**Total Agent Definitions:** 9
**Total Lines of Agent Specs:** 2,533
**Roadmap Features Covered:** 5/5
**Status:** ✅ Complete - Ready for Implementation
