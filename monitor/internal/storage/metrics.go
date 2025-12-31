package storage

import (
	"sync"
	"time"
)

// MetricPoint represents a single metric data point
type MetricPoint struct {
	Timestamp  int64              `json:"timestamp"`
	MetricType string             `json:"metric_type"`
	MetricName string             `json:"metric_name"`
	Value      float64            `json:"value"`
	Labels     map[string]string  `json:"labels"`
}

// MetricStore holds metrics in memory with a ring buffer per metric
type MetricStore struct {
	mu       sync.RWMutex
	metrics  map[string]*RingBuffer // key: metric_name
	capacity int
}

// RingBuffer implements a fixed-size circular buffer for time-series data
type RingBuffer struct {
	data     []MetricPoint
	capacity int
	head     int
	size     int
}

// NewRingBuffer creates a new ring buffer with specified capacity
func NewRingBuffer(capacity int) *RingBuffer {
	return &RingBuffer{
		data:     make([]MetricPoint, capacity),
		capacity: capacity,
		head:     0,
		size:     0,
	}
}

// Append adds a metric point to the ring buffer
func (rb *RingBuffer) Append(point MetricPoint) {
	rb.data[rb.head] = point
	rb.head = (rb.head + 1) % rb.capacity
	if rb.size < rb.capacity {
		rb.size++
	}
}

// GetLast returns the last n points (newest first)
func (rb *RingBuffer) GetLast(n int) []MetricPoint {
	if n > rb.size {
		n = rb.size
	}

	result := make([]MetricPoint, n)
	idx := rb.head - 1
	if idx < 0 {
		idx = rb.size - 1
	}

	for i := 0; i < n; i++ {
		if idx < 0 {
			idx = rb.size - 1
		}
		result[i] = rb.data[idx]
		idx--
	}

	return result
}

// GetRange returns points within a time range
func (rb *RingBuffer) GetRange(startMs, endMs int64) []MetricPoint {
	result := make([]MetricPoint, 0, rb.size)

	// Iterate through all valid data
	for i := 0; i < rb.size; i++ {
		idx := (rb.head - rb.size + i + rb.capacity) % rb.capacity
		point := rb.data[idx]
		if point.Timestamp >= startMs && point.Timestamp <= endMs {
			result = append(result, point)
		}
	}

	return result
}

// NewMetricStore creates a new metric store
func NewMetricStore(capacity int) *MetricStore {
	return &MetricStore{
		metrics:  make(map[string]*RingBuffer),
		capacity: capacity,
	}
}

// Append adds a metric point to the store
func (ms *MetricStore) Append(point MetricPoint) {
	ms.mu.Lock()
	defer ms.mu.Unlock()

	// Create ring buffer for new metrics
	if _, exists := ms.metrics[point.MetricName]; !exists {
		ms.metrics[point.MetricName] = NewRingBuffer(ms.capacity)
	}

	ms.metrics[point.MetricName].Append(point)
}

// GetLast returns the last n points for a specific metric
func (ms *MetricStore) GetLast(metricName string, n int) []MetricPoint {
	ms.mu.RLock()
	defer ms.mu.RUnlock()

	rb, exists := ms.metrics[metricName]
	if !exists {
		return []MetricPoint{}
	}

	return rb.GetLast(n)
}

// GetRange returns points within a time range for a specific metric
func (ms *MetricStore) GetRange(metricName string, startMs, endMs int64) []MetricPoint {
	ms.mu.RLock()
	defer ms.mu.RUnlock()

	rb, exists := ms.metrics[metricName]
	if !exists {
		return []MetricPoint{}
	}

	return rb.GetRange(startMs, endMs)
}

// GetAllMetrics returns all metric names
func (ms *MetricStore) GetAllMetrics() []string {
	ms.mu.RLock()
	defer ms.mu.RUnlock()

	names := make([]string, 0, len(ms.metrics))
	for name := range ms.metrics {
		names = append(names, name)
	}
	return names
}

// GetRecentMetrics returns recent points from all metrics (for dashboard)
func (ms *MetricStore) GetRecentMetrics(durationMs int64) map[string][]MetricPoint {
	ms.mu.RLock()
	defer ms.mu.RUnlock()

	endMs := time.Now().UnixMilli()
	startMs := endMs - durationMs

	result := make(map[string][]MetricPoint)
	for name, rb := range ms.metrics {
		points := rb.GetRange(startMs, endMs)
		if len(points) > 0 {
			result[name] = points
		}
	}

	return result
}
