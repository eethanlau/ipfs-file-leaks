package main

import (
	"fmt"
	"google.golang.org/grpc"
	"log"
	"net"
)

// gRPC server entry point in main.go
func main() {
	// Listen on a TCP port
	lis, err := net.Listen("tcp", ":50051")
	if err != nil {
		log.Fatalf("failed to listen: %v", err)
	}
	// Create a new gRPC server instance
	s := grpc.NewServer()

	// Register the KeyServer implementation with the gRPC server
	pb.RegisterKeyServiceServer(s, &server{})
	log.Printf("server listening at %v", lis.Addr())

	// serving requests
	if err := s.Serve(lis); err != nil {
		log.Fatalf("failed to serve: %v", err)
	}

}
