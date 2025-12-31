-- Migration: 001_initial_schema
-- Description: Initial schema creation for metrics and execution tables
-- Created: 2025-12-31

-- This migration creates the base hypertables and indexes
-- It is idempotent and safe to run multiple times

BEGIN;

-- Enable TimescaleDB extension
CREATE EXTENSION IF NOT EXISTS timescaledb;

-- Create metrics table (if not exists)
CREATE TABLE IF NOT EXISTS metrics (
    timestamp TIMESTAMPTZ NOT NULL,
    metric_name TEXT NOT NULL,
    value DOUBLE PRECISION NOT NULL,
    labels JSONB DEFAULT '{}'::jsonb
);

-- Create hypertable if not already created
SELECT create_hypertable('metrics', 'timestamp',
    chunk_time_interval => INTERVAL '1 day',
    if_not_exists => TRUE
);

-- Create orders table (if not exists)
CREATE TABLE IF NOT EXISTS orders (
    id UUID PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL,
    venue TEXT NOT NULL,
    market TEXT NOT NULL,
    side TEXT NOT NULL,
    order_type TEXT NOT NULL,
    price DOUBLE PRECISION,
    size DOUBLE PRECISION NOT NULL,
    status TEXT NOT NULL,
    client_order_id TEXT UNIQUE NOT NULL,
    venue_order_id TEXT,
    time_in_force TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

SELECT create_hypertable('orders', 'timestamp',
    chunk_time_interval => INTERVAL '1 day',
    if_not_exists => TRUE
);

-- Create fills table (if not exists)
CREATE TABLE IF NOT EXISTS fills (
    id UUID PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL,
    order_id UUID NOT NULL,
    venue TEXT NOT NULL,
    market TEXT NOT NULL,
    side TEXT NOT NULL,
    price DOUBLE PRECISION NOT NULL,
    size DOUBLE PRECISION NOT NULL,
    fee DOUBLE PRECISION NOT NULL,
    fee_currency TEXT NOT NULL,
    trade_id TEXT,
    liquidity TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

SELECT create_hypertable('fills', 'timestamp',
    chunk_time_interval => INTERVAL '1 day',
    if_not_exists => TRUE
);

-- Create positions table (if not exists)
CREATE TABLE IF NOT EXISTS positions (
    timestamp TIMESTAMPTZ NOT NULL,
    market TEXT NOT NULL,
    venue TEXT NOT NULL,
    size DOUBLE PRECISION NOT NULL,
    avg_entry_price DOUBLE PRECISION NOT NULL,
    unrealized_pnl DOUBLE PRECISION,
    realized_pnl DOUBLE PRECISION,
    mark_price DOUBLE PRECISION,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

SELECT create_hypertable('positions', 'timestamp',
    chunk_time_interval => INTERVAL '1 day',
    if_not_exists => TRUE
);

COMMIT;
