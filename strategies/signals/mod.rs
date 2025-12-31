//! Signal generation framework

pub mod technical;
pub mod microstructure;
pub mod composite;

pub use technical::{
    SimpleMovingAverage,
    ExponentialMovingAverage,
    RelativeStrengthIndex,
    BollingerBands,
    MovingAverageConvergenceDivergence,
};

pub use microstructure::{
    OrderImbalance,
    SpreadAnalyzer,
};

pub use composite::{
    CompositeSignal,
    SignalAggregator,
};
