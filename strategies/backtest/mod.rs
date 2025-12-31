//! Backtesting engine for strategy validation

pub mod engine;
pub mod fill_simulator;

pub use engine::{BacktestEngine, BacktestConfig, BacktestResult};
pub use fill_simulator::{FillSimulator, FillSimulatorConfig};
