package main

import (
	"log"
	"net"

	pb "ipfs-file-leaks/key-server/pb"
	"ipfs-file-leaks/key-server/server"

	"google.golang.org/grpc"
)

// gRPC key server entry point
func main() {
	// Listen on TCP port 50051
	lis, err := net.Listen("tcp", ":50051")
	if err != nil {
		log.Fatalf("failed to listen: %v", err)
	}

	// Create a new gRPC server instance
	s := grpc.NewServer()

	// Register the KeyServer implementation with the gRPC server
	pb.RegisterKeyServiceServer(s, server.NewServer())
	log.Printf("Key server listening at %v", lis.Addr())

	// Start serving requests
	if err := s.Serve(lis); err != nil {
		log.Fatalf("failed to serve: %v", err)
	}
}
