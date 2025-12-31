package storage

import (
	"testing"
	"time"
)

func TestRingBuffer_Append(t *testing.T) {
	rb := NewRingBuffer(3)

	// Add points
	rb.Append(MetricPoint{Timestamp: 1000, Value: 10.0})
	rb.Append(MetricPoint{Timestamp: 2000, Value: 20.0})
	rb.Append(MetricPoint{Timestamp: 3000, Value: 30.0})

	if rb.size != 3 {
		t.Errorf("Expected size 3, got %d", rb.size)
	}

	// Add one more (should wrap)
	rb.Append(MetricPoint{Timestamp: 4000, Value: 40.0})

	if rb.size != 3 {
		t.Errorf("Expected size 3 after wrap, got %d", rb.size)
	}
}

func TestRingBuffer_GetLast(t *testing.T) {
	rb := NewRingBuffer(5)

	// Add points
	for i := 1; i <= 5; i++ {
		rb.Append(MetricPoint{Timestamp: int64(i * 1000), Value: float64(i * 10)})
	}

	// Get last 3
	points := rb.GetLast(3)

	if len(points) != 3 {
		t.Errorf("Expected 3 points, got %d", len(points))
	}

	// Should be newest first
	if points[0].Timestamp != 5000 {
		t.Errorf("Expected newest timestamp 5000, got %d", points[0].Timestamp)
	}
	if points[2].Timestamp != 3000 {
		t.Errorf("Expected oldest timestamp 3000, got %d", points[2].Timestamp)
	}
}

func TestRingBuffer_GetRange(t *testing.T) {
	rb := NewRingBuffer(10)

	// Add points with timestamps from 1000 to 5000
	for i := 1; i <= 5; i++ {
		rb.Append(MetricPoint{Timestamp: int64(i * 1000), Value: float64(i * 10)})
	}

	// Get range [2000, 4000]
	points := rb.GetRange(2000, 4000)

	if len(points) != 3 {
		t.Errorf("Expected 3 points in range, got %d", len(points))
	}

	// Verify timestamps
	for _, p := range points {
		if p.Timestamp < 2000 || p.Timestamp > 4000 {
			t.Errorf("Point timestamp %d outside range [2000, 4000]", p.Timestamp)
		}
	}
}

func TestRingBuffer_Wrapping(t *testing.T) {
	rb := NewRingBuffer(3)

	// Add 5 points (should keep last 3)
	for i := 1; i <= 5; i++ {
		rb.Append(MetricPoint{Timestamp: int64(i * 1000), Value: float64(i * 10)})
	}

	points := rb.GetLast(3)

	if len(points) != 3 {
		t.Errorf("Expected 3 points, got %d", len(points))
	}

	// Should have timestamps 5000, 4000, 3000
	if points[0].Timestamp != 5000 {
		t.Errorf("Expected timestamp 5000, got %d", points[0].Timestamp)
	}
	if points[2].Timestamp != 3000 {
		t.Errorf("Expected timestamp 3000, got %d", points[2].Timestamp)
	}
}

func TestMetricStore_Append(t *testing.T) {
	store := NewMetricStore(100)

	metric := MetricPoint{
		Timestamp:  time.Now().UnixMilli(),
		MetricType: "gauge",
		MetricName: "test.metric",
		Value:      42.0,
		Labels:     map[string]string{"key": "value"},
	}

	store.Append(metric)

	points := store.GetLast("test.metric", 1)
	if len(points) != 1 {
		t.Errorf("Expected 1 point, got %d", len(points))
	}

	if points[0].Value != 42.0 {
		t.Errorf("Expected value 42.0, got %f", points[0].Value)
	}
}

func TestMetricStore_MultipleMetrics(t *testing.T) {
	store := NewMetricStore(100)

	// Add different metrics
	store.Append(MetricPoint{
		Timestamp:  1000,
		MetricName: "metric.one",
		Value:      10.0,
	})
	store.Append(MetricPoint{
		Timestamp:  2000,
		MetricName: "metric.two",
		Value:      20.0,
	})

	// Verify both metrics stored separately
	names := store.GetAllMetrics()
	if len(names) != 2 {
		t.Errorf("Expected 2 metrics, got %d", len(names))
	}

	points1 := store.GetLast("metric.one", 10)
	points2 := store.GetLast("metric.two", 10)

	if len(points1) != 1 || len(points2) != 1 {
		t.Errorf("Expected 1 point for each metric")
	}

	if points1[0].Value != 10.0 {
		t.Errorf("Expected value 10.0 for metric.one, got %f", points1[0].Value)
	}
	if points2[0].Value != 20.0 {
		t.Errorf("Expected value 20.0 for metric.two, got %f", points2[0].Value)
	}
}

func TestMetricStore_GetRecentMetrics(t *testing.T) {
	store := NewMetricStore(100)

	now := time.Now().UnixMilli()

	// Add old metric (outside window)
	store.Append(MetricPoint{
		Timestamp:  now - 120000, // 2 minutes ago
		MetricName: "old.metric",
		Value:      100.0,
	})

	// Add recent metric (inside window)
	store.Append(MetricPoint{
		Timestamp:  now - 30000, // 30 seconds ago
		MetricName: "recent.metric",
		Value:      200.0,
	})

	// Get last 60 seconds
	recent := store.GetRecentMetrics(60000)

	// Should only have recent.metric
	if _, ok := recent["recent.metric"]; !ok {
		t.Error("Expected recent.metric to be in results")
	}

	if _, ok := recent["old.metric"]; ok {
		t.Error("Expected old.metric to NOT be in results")
	}
}

func TestMetricStore_GetNonExistent(t *testing.T) {
	store := NewMetricStore(100)

	points := store.GetLast("nonexistent", 10)
	if len(points) != 0 {
		t.Errorf("Expected 0 points for nonexistent metric, got %d", len(points))
	}

	rangePoints := store.GetRange("nonexistent", 1000, 2000)
	if len(rangePoints) != 0 {
		t.Errorf("Expected 0 points for nonexistent metric range, got %d", len(rangePoints))
	}
}

func BenchmarkRingBuffer_Append(b *testing.B) {
	rb := NewRingBuffer(1000)
	metric := MetricPoint{Timestamp: 1000, Value: 42.0}

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		rb.Append(metric)
	}
}

func BenchmarkMetricStore_Append(b *testing.B) {
	store := NewMetricStore(1000)
	metric := MetricPoint{
		Timestamp:  1000,
		MetricName: "test.metric",
		Value:      42.0,
	}

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		store.Append(metric)
	}
}
