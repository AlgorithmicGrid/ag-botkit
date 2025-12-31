package server

import (
	"log"
	"net/http"
	"time"

	"github.com/ag-botkit/monitor/internal/storage"
)

// Server represents the HTTP server
type Server struct {
	store  *storage.MetricStore
	hub    *Hub
	server *http.Server
}

// NewServer creates a new HTTP server
func NewServer(addr string, store *storage.MetricStore) *Server {
	hub := NewHub(store)

	s := &Server{
		store: store,
		hub:   hub,
		server: &http.Server{
			Addr:         addr,
			ReadTimeout:  15 * time.Second,
			WriteTimeout: 15 * time.Second,
			IdleTimeout:  60 * time.Second,
		},
	}

	// Start hub
	go hub.Run()

	return s
}

// SetupRoutes configures HTTP routes
func (s *Server) SetupRoutes(staticFS http.FileSystem) {
	mux := http.NewServeMux()

	// WebSocket endpoints
	mux.HandleFunc("/metrics", s.hub.HandleMetricsWS)
	mux.HandleFunc("/dashboard", s.hub.HandleDashboardWS)

	// Static files
	mux.Handle("/", http.FileServer(staticFS))

	s.server.Handler = s.loggingMiddleware(mux)
}

// Start starts the HTTP server
func (s *Server) Start() error {
	log.Printf("Starting monitor server on %s", s.server.Addr)
	log.Printf("Dashboard: http://%s", s.server.Addr)
	log.Printf("Metrics WS: ws://%s/metrics", s.server.Addr)
	log.Printf("Dashboard WS: ws://%s/dashboard", s.server.Addr)
	return s.server.ListenAndServe()
}

// loggingMiddleware logs HTTP requests
func (s *Server) loggingMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		start := time.Now()
		next.ServeHTTP(w, r)
		log.Printf("%s %s %v", r.Method, r.URL.Path, time.Since(start))
	})
}
