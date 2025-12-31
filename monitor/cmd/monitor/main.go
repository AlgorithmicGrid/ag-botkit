package main

import (
	"flag"
	"log"
	"net/http"
	"os"
	"path/filepath"

	"github.com/ag-botkit/monitor/internal/server"
	"github.com/ag-botkit/monitor/internal/storage"
)

func main() {
	addr := flag.String("addr", "localhost:8080", "HTTP server address")
	capacity := flag.Int("capacity", 10000, "Metric storage capacity per metric")
	webDir := flag.String("web", "", "Web directory path (defaults to ./web or ../../web)")
	flag.Parse()

	log.SetFlags(log.LstdFlags | log.Lshortfile)

	// Determine web directory
	var webPath string
	if *webDir != "" {
		webPath = *webDir
	} else {
		// Try to find web directory
		candidates := []string{
			"./web",
			"../../web",
			filepath.Join(filepath.Dir(os.Args[0]), "../../web"),
		}
		for _, candidate := range candidates {
			if info, err := os.Stat(candidate); err == nil && info.IsDir() {
				webPath = candidate
				break
			}
		}
	}

	if webPath == "" {
		log.Fatal("Could not find web directory. Use -web flag to specify path.")
	}

	absPath, _ := filepath.Abs(webPath)
	log.Printf("Serving static files from: %s", absPath)

	// Create metric store
	store := storage.NewMetricStore(*capacity)

	// Create and start server
	srv := server.NewServer(*addr, store)
	srv.SetupRoutes(http.Dir(webPath))

	log.Fatal(srv.Start())
}
