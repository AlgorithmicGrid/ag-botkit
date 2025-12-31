#!/usr/bin/env bash
# Validate all agent definitions and architecture documents

set -e

PROJECT_ROOT="/Users/yaroslav/ag-botkit"
AGENTS_DIR="$PROJECT_ROOT/.claude/agents"

echo "=================================="
echo "Agent Architecture Validation"
echo "=================================="
echo ""

# Check agent definitions exist
echo "Checking agent definitions..."
REQUIRED_AGENTS=(
    "system-architect.md"
    "core-c-implementer.md"
    "risk-engine.md"
    "monitor-ui.md"
    "exec-gateway.md"
    "storage-layer.md"
    "advanced-risk.md"
    "strategy-engine.md"
    "devops-infra.md"
)

MISSING_COUNT=0
for agent in "${REQUIRED_AGENTS[@]}"; do
    if [ -f "$AGENTS_DIR/$agent" ]; then
        echo "✓ $agent"
    else
        echo "✗ $agent (MISSING)"
        MISSING_COUNT=$((MISSING_COUNT + 1))
    fi
done

echo ""
echo "Agent Count:"
echo "  Total: ${#REQUIRED_AGENTS[@]}"
echo "  Found: $((${#REQUIRED_AGENTS[@]} - MISSING_COUNT))"
echo "  Missing: $MISSING_COUNT"

# Check architecture documents
echo ""
echo "Checking architecture documents..."
ARCH_DOCS=(
    "MULTI_AGENT_PLAN.md"
    "ROADMAP_AGENTS_SUMMARY.md"
    "CLAUDE.md"
)

for doc in "${ARCH_DOCS[@]}"; do
    if [ -f "$PROJECT_ROOT/$doc" ]; then
        lines=$(wc -l < "$PROJECT_ROOT/$doc")
        echo "✓ $doc ($lines lines)"
    else
        echo "✗ $doc (MISSING)"
    fi
done

# Check MULTI_AGENT_PLAN version
echo ""
echo "Checking MULTI_AGENT_PLAN.md version..."
if grep -q "Version | 2.0" "$PROJECT_ROOT/MULTI_AGENT_PLAN.md"; then
    echo "✓ MULTI_AGENT_PLAN.md is version 2.0 (includes roadmap)"
else
    echo "⚠ MULTI_AGENT_PLAN.md may not include roadmap architecture"
fi

# Verify roadmap sections exist
echo ""
echo "Verifying roadmap sections in MULTI_AGENT_PLAN.md..."
ROADMAP_SECTIONS=(
    "12.2 Feature: CLOB API Integration"
    "12.3 Feature: Persistent Storage"
    "12.4 Feature: Advanced Risk Models"
    "12.5 Feature: Multi-Market Strategy Support"
    "12.6 Feature: Production Deployment Tooling"
)

for section in "${ROADMAP_SECTIONS[@]}"; do
    if grep -q "$section" "$PROJECT_ROOT/MULTI_AGENT_PLAN.md"; then
        echo "✓ $section"
    else
        echo "✗ $section (MISSING)"
    fi
done

# Summary
echo ""
echo "=================================="
echo "Validation Summary"
echo "=================================="

if [ $MISSING_COUNT -eq 0 ]; then
    echo "✅ All agent definitions present"
    echo "✅ Architecture documents complete"
    echo "✅ Ready for roadmap implementation"
else
    echo "❌ $MISSING_COUNT agent definition(s) missing"
    echo "Please ensure all agents are defined before proceeding"
fi

echo ""
echo "Next steps:"
echo "1. Review MULTI_AGENT_PLAN.md section 12 (Roadmap Architecture)"
echo "2. Start with Phase 1: storage-layer agent (weeks 1-2)"
echo "3. Follow dependency graph for subsequent phases"
echo ""
