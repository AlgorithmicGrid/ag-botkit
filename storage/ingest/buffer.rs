use crate::types::MetricPoint;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Buffered metric storage for high-throughput ingestion
pub struct MetricBuffer {
    buffer: Arc<RwLock<Vec<MetricPoint>>>,
    capacity: usize,
}

impl MetricBuffer {
    /// Create a new metric buffer with given capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: Arc::new(RwLock::new(Vec::with_capacity(capacity))),
            capacity,
        }
    }

    /// Add a metric to the buffer
    pub async fn push(&self, metric: MetricPoint) -> Result<(), String> {
        let mut buffer = self.buffer.write().await;

        if buffer.len() >= self.capacity {
            return Err("Buffer full".to_string());
        }

        buffer.push(metric);
        Ok(())
    }

    /// Drain all metrics from the buffer
    pub async fn drain(&self) -> Vec<MetricPoint> {
        let mut buffer = self.buffer.write().await;
        std::mem::take(&mut *buffer)
    }

    /// Get current buffer size
    pub async fn len(&self) -> usize {
        self.buffer.read().await.len()
    }

    /// Check if buffer is empty
    pub async fn is_empty(&self) -> bool {
        self.buffer.read().await.is_empty()
    }

    /// Get buffer capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_buffer() {
        let buffer = MetricBuffer::new(10);

        let metric = MetricPoint::new("test", 1.0);
        buffer.push(metric).await.unwrap();

        assert_eq!(buffer.len().await, 1);

        let drained = buffer.drain().await;
        assert_eq!(drained.len(), 1);
        assert_eq!(buffer.len().await, 0);
    }
}
