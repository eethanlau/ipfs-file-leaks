package main

import (
	"log"
	"net"
	"os"
	"strings"

	pb "ipfs-file-leaks/key-server/pb"
	"ipfs-file-leaks/key-server/server"

	"google.golang.org/grpc"
)

// gRPC key server entry point.
//
// Replication peers are read from the KEY_SERVER_PEERS environment
// variable as a comma-separated list of "host:port" addresses. Empty or
// unset means single-server mode (no replication).
func main() {
	lis, err := net.Listen("tcp", ":50051")
	if err != nil {
		log.Fatalf("failed to listen: %v", err)
	}

	peers := parsePeers(os.Getenv("KEY_SERVER_PEERS"))
	if len(peers) > 0 {
		log.Printf("Replicating to peers: %v", peers)
	} else {
		log.Printf("No peers configured; running in single-server mode")
	}

	s := grpc.NewServer()
	pb.RegisterKeyServiceServer(s, server.NewServer(peers))
	log.Printf("Key server listening at %v", lis.Addr())

	if err := s.Serve(lis); err != nil {
		log.Fatalf("failed to serve: %v", err)
	}
}

func parsePeers(raw string) []string {
	if raw == "" {
		return nil
	}
	parts := strings.Split(raw, ",")
	peers := make([]string, 0, len(parts))
	for _, p := range parts {
		if p = strings.TrimSpace(p); p != "" {
			peers = append(peers, p)
		}
	}
	return peers
}
