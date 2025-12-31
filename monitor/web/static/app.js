// WebSocket connection
let ws = null;
let reconnectInterval = 2000;
let reconnectTimer = null;

// Data buffers for charts (last 60 seconds)
const MAX_POINTS = 120;
const buffers = {
    lag: { timestamps: [], values: [] },
    mps: { timestamps: [], values: [] },
    messages: { timestamps: [], values: [] },
    position: { timestamps: [], values: [] },
    risk: { timestamps: [], values: [] }
};

// Statistics
const stats = {
    lag: { current: 0, sum: 0, count: 0, max: 0 },
    mps: { current: 0, sum: 0, count: 0, max: 0 },
    messages: { total: 0, lastUpdate: Date.now() },
    risk: { allowed: 0, blocked: 0 }
};

// uPlot instances
let charts = {
    lag: null,
    mps: null,
    messages: null,
    position: null,
    risk: null
};

// Initialize charts
function initCharts() {
    // Common options
    const commonOpts = {
        width: 600,
        height: 250,
        series: [
            {},
            {
                stroke: '#667eea',
                width: 2,
                fill: 'rgba(102, 126, 234, 0.1)'
            }
        ],
        axes: [
            {
                stroke: '#888',
                grid: { stroke: '#eee' }
            },
            {
                stroke: '#888',
                grid: { stroke: '#eee' }
            }
        ],
        legend: {
            show: false
        }
    };

    // RTDS Lag chart
    charts.lag = new uPlot(
        {
            ...commonOpts,
            title: 'Lag (ms)',
            scales: {
                x: { time: true },
                y: { auto: true }
            }
        },
        [[0], [0]],
        document.getElementById('lag-chart')
    );

    // Messages per second chart
    charts.mps = new uPlot(
        {
            ...commonOpts,
            title: 'Messages/sec',
            scales: {
                x: { time: true },
                y: { auto: true }
            }
        },
        [[0], [0]],
        document.getElementById('mps-chart')
    );

    // Messages received chart
    charts.messages = new uPlot(
        {
            ...commonOpts,
            title: 'Total Messages',
            scales: {
                x: { time: true },
                y: { auto: true }
            },
            series: [
                {},
                {
                    stroke: '#10b981',
                    width: 2,
                    fill: 'rgba(16, 185, 129, 0.1)'
                }
            ]
        },
        [[0], [0]],
        document.getElementById('messages-chart')
    );

    // Position size chart
    charts.position = new uPlot(
        {
            ...commonOpts,
            title: 'Position',
            scales: {
                x: { time: true },
                y: { auto: true }
            },
            series: [
                {},
                {
                    stroke: '#f59e0b',
                    width: 2,
                    fill: 'rgba(245, 158, 11, 0.1)'
                }
            ]
        },
        [[0], [0]],
        document.getElementById('position-chart')
    );

    // Risk decisions chart
    charts.risk = new uPlot(
        {
            ...commonOpts,
            title: 'Risk (1=allowed, 0=blocked)',
            scales: {
                x: { time: true },
                y: { min: -0.1, max: 1.1 }
            },
            series: [
                {},
                {
                    stroke: '#8b5cf6',
                    width: 2,
                    points: { show: true, size: 5 }
                }
            ]
        },
        [[0], [0]],
        document.getElementById('risk-chart')
    );

    // Handle window resize
    window.addEventListener('resize', () => {
        Object.values(charts).forEach(chart => {
            if (chart) {
                const parent = chart.root.parentElement;
                chart.setSize({ width: parent.clientWidth, height: 250 });
            }
        });
    });
}

// Update chart with new data
function updateChart(chartName, timestamp, value) {
    const buffer = buffers[chartName];
    const chart = charts[chartName];

    if (!buffer || !chart) return;

    // Add new point
    buffer.timestamps.push(timestamp / 1000); // Convert to seconds for uPlot
    buffer.values.push(value);

    // Keep only last MAX_POINTS
    if (buffer.timestamps.length > MAX_POINTS) {
        buffer.timestamps.shift();
        buffer.values.shift();
    }

    // Update chart
    chart.setData([buffer.timestamps, buffer.values]);
}

// Update statistics display
function updateStats(metricName, value) {
    switch (metricName) {
        case 'polymarket.rtds.lag_ms':
            stats.lag.current = value;
            stats.lag.sum += value;
            stats.lag.count++;
            stats.lag.max = Math.max(stats.lag.max, value);
            document.getElementById('lag-current').textContent = value.toFixed(1);
            document.getElementById('lag-avg').textContent = (stats.lag.sum / stats.lag.count).toFixed(1);
            document.getElementById('lag-max').textContent = stats.lag.max.toFixed(1);
            break;

        case 'polymarket.rtds.msgs_per_second':
            stats.mps.current = value;
            stats.mps.sum += value;
            stats.mps.count++;
            stats.mps.max = Math.max(stats.mps.max, value);
            document.getElementById('mps-current').textContent = value.toFixed(1);
            document.getElementById('mps-avg').textContent = (stats.mps.sum / stats.mps.count).toFixed(1);
            document.getElementById('mps-max').textContent = stats.mps.max.toFixed(1);
            break;

        case 'polymarket.rtds.messages_received':
            stats.messages.total += value;
            const now = Date.now();
            const elapsed = (now - stats.messages.lastUpdate) / 1000;
            const rate = elapsed > 0 ? value / elapsed : 0;
            stats.messages.lastUpdate = now;
            document.getElementById('msg-total').textContent = stats.messages.total;
            document.getElementById('msg-rate').textContent = rate.toFixed(1);
            break;

        case 'polymarket.risk.decision':
            if (value === 1) {
                stats.risk.allowed++;
            } else {
                stats.risk.blocked++;
            }
            document.getElementById('risk-allowed').textContent = stats.risk.allowed;
            document.getElementById('risk-blocked').textContent = stats.risk.blocked;
            break;

        case 'polymarket.risk.kill_switch':
            const indicator = document.getElementById('kill-switch-indicator');
            const text = document.getElementById('kill-switch-text');
            if (value === 1) {
                indicator.className = 'kill-switch-indicator on';
                text.textContent = 'ON';
            } else {
                indicator.className = 'kill-switch-indicator off';
                text.textContent = 'OFF';
            }
            break;

        case 'polymarket.position.size':
            const total = value;
            document.getElementById('pos-total').textContent = total.toFixed(2);
            break;
    }
}

// Process incoming metric
function processMetric(metric) {
    const { timestamp, metric_name, value } = metric;

    // Update statistics
    updateStats(metric_name, value);

    // Update charts
    switch (metric_name) {
        case 'polymarket.rtds.lag_ms':
            updateChart('lag', timestamp, value);
            break;

        case 'polymarket.rtds.msgs_per_second':
            updateChart('mps', timestamp, value);
            break;

        case 'polymarket.rtds.messages_received':
            updateChart('messages', timestamp, stats.messages.total);
            break;

        case 'polymarket.position.size':
            updateChart('position', timestamp, value);
            break;

        case 'polymarket.risk.decision':
            updateChart('risk', timestamp, value);
            break;
    }
}

// WebSocket connection management
function connect() {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${protocol}//${window.location.host}/dashboard`;

    console.log('Connecting to', wsUrl);
    ws = new WebSocket(wsUrl);

    ws.onopen = () => {
        console.log('WebSocket connected');
        document.getElementById('status-indicator').className = 'status-dot connected';
        document.getElementById('status-text').textContent = 'Connected';

        if (reconnectTimer) {
            clearTimeout(reconnectTimer);
            reconnectTimer = null;
        }
    };

    ws.onmessage = (event) => {
        try {
            const metric = JSON.parse(event.data);
            processMetric(metric);
        } catch (err) {
            console.error('Error parsing metric:', err, event.data);
        }
    };

    ws.onerror = (error) => {
        console.error('WebSocket error:', error);
    };

    ws.onclose = () => {
        console.log('WebSocket disconnected');
        document.getElementById('status-indicator').className = 'status-dot disconnected';
        document.getElementById('status-text').textContent = 'Disconnected';

        // Attempt reconnection
        if (!reconnectTimer) {
            reconnectTimer = setTimeout(() => {
                console.log('Attempting reconnection...');
                connect();
            }, reconnectInterval);
        }
    };
}

// Initialize on page load
document.addEventListener('DOMContentLoaded', () => {
    console.log('Initializing dashboard...');
    initCharts();
    connect();
});
