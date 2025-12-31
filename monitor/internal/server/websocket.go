package server

import (
	"encoding/json"
	"log"
	"net/http"
	"sync"
	"time"

	"github.com/ag-botkit/monitor/internal/storage"
	"github.com/gorilla/websocket"
)

var upgrader = websocket.Upgrader{
	ReadBufferSize:  1024,
	WriteBufferSize: 1024,
	CheckOrigin: func(r *http.Request) bool {
		return true // Allow all origins for local development
	},
}

// Hub maintains active WebSocket connections and broadcasts metrics
type Hub struct {
	store       *storage.MetricStore
	clients     map[*Client]bool
	broadcast   chan *storage.MetricPoint
	register    chan *Client
	unregister  chan *Client
	mu          sync.RWMutex
}

// Client represents a WebSocket connection
type Client struct {
	hub  *Hub
	conn *websocket.Conn
	send chan []byte
}

// NewHub creates a new WebSocket hub
func NewHub(store *storage.MetricStore) *Hub {
	return &Hub{
		store:      store,
		clients:    make(map[*Client]bool),
		broadcast:  make(chan *storage.MetricPoint, 256),
		register:   make(chan *Client),
		unregister: make(chan *Client),
	}
}

// Run starts the hub's main loop
func (h *Hub) Run() {
	for {
		select {
		case client := <-h.register:
			h.mu.Lock()
			h.clients[client] = true
			h.mu.Unlock()
			log.Printf("Dashboard client connected (total: %d)", len(h.clients))

		case client := <-h.unregister:
			h.mu.Lock()
			if _, ok := h.clients[client]; ok {
				delete(h.clients, client)
				close(client.send)
			}
			h.mu.Unlock()
			log.Printf("Dashboard client disconnected (total: %d)", len(h.clients))

		case metric := <-h.broadcast:
			// Broadcast to all dashboard clients
			data, err := json.Marshal(metric)
			if err != nil {
				log.Printf("Error marshaling metric: %v", err)
				continue
			}

			h.mu.RLock()
			for client := range h.clients {
				select {
				case client.send <- data:
				default:
					// Client is slow, close it
					close(client.send)
					delete(h.clients, client)
				}
			}
			h.mu.RUnlock()
		}
	}
}

// BroadcastMetric sends a metric to all connected dashboard clients
func (h *Hub) BroadcastMetric(metric *storage.MetricPoint) {
	select {
	case h.broadcast <- metric:
	default:
		log.Printf("Warning: broadcast channel full, dropping metric")
	}
}

// HandleMetricsWS handles the /metrics WebSocket endpoint (ingestion)
func (h *Hub) HandleMetricsWS(w http.ResponseWriter, r *http.Request) {
	conn, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		log.Printf("Error upgrading metrics connection: %v", err)
		return
	}
	defer conn.Close()

	log.Printf("Metrics client connected from %s", r.RemoteAddr)

	for {
		_, message, err := conn.ReadMessage()
		if err != nil {
			if websocket.IsUnexpectedCloseError(err, websocket.CloseGoingAway, websocket.CloseAbnormalClosure) {
				log.Printf("Metrics WebSocket error: %v", err)
			}
			break
		}

		// Parse metric
		var metric storage.MetricPoint
		if err := json.Unmarshal(message, &metric); err != nil {
			log.Printf("Error parsing metric: %v (message: %s)", err, string(message))
			continue
		}

		// Store metric
		h.store.Append(metric)

		// Broadcast to dashboard clients
		h.BroadcastMetric(&metric)
	}

	log.Printf("Metrics client disconnected from %s", r.RemoteAddr)
}

// HandleDashboardWS handles the /dashboard WebSocket endpoint (broadcast)
func (h *Hub) HandleDashboardWS(w http.ResponseWriter, r *http.Request) {
	conn, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		log.Printf("Error upgrading dashboard connection: %v", err)
		return
	}

	client := &Client{
		hub:  h,
		conn: conn,
		send: make(chan []byte, 256),
	}

	h.register <- client

	// Start goroutines for reading and writing
	go client.writePump()
	go client.readPump()

	// Send initial data (last 60 seconds)
	go func() {
		time.Sleep(100 * time.Millisecond)
		recentMetrics := h.store.GetRecentMetrics(60000) // Last 60 seconds

		for metricName, points := range recentMetrics {
			for _, point := range points {
				data, err := json.Marshal(point)
				if err != nil {
					continue
				}
				select {
				case client.send <- data:
				default:
					log.Printf("Warning: initial data send failed for %s", metricName)
				}
			}
		}
	}()
}

// readPump reads messages from the WebSocket connection
func (c *Client) readPump() {
	defer func() {
		c.hub.unregister <- c
		c.conn.Close()
	}()

	c.conn.SetReadDeadline(time.Now().Add(60 * time.Second))
	c.conn.SetPongHandler(func(string) error {
		c.conn.SetReadDeadline(time.Now().Add(60 * time.Second))
		return nil
	})

	for {
		_, _, err := c.conn.ReadMessage()
		if err != nil {
			if websocket.IsUnexpectedCloseError(err, websocket.CloseGoingAway, websocket.CloseAbnormalClosure) {
				log.Printf("Dashboard WebSocket error: %v", err)
			}
			break
		}
	}
}

// writePump writes messages to the WebSocket connection
func (c *Client) writePump() {
	ticker := time.NewTicker(54 * time.Second)
	defer func() {
		ticker.Stop()
		c.conn.Close()
	}()

	for {
		select {
		case message, ok := <-c.send:
			c.conn.SetWriteDeadline(time.Now().Add(10 * time.Second))
			if !ok {
				// Hub closed the channel
				c.conn.WriteMessage(websocket.CloseMessage, []byte{})
				return
			}

			if err := c.conn.WriteMessage(websocket.TextMessage, message); err != nil {
				return
			}

		case <-ticker.C:
			c.conn.SetWriteDeadline(time.Now().Add(10 * time.Second))
			if err := c.conn.WriteMessage(websocket.PingMessage, nil); err != nil {
				return
			}
		}
	}
}
