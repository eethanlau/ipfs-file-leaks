package server

import (
	"context"
	pb "ipfs-file-leaks/key-server/pb" // The auto-generated proto file import
)

// server is used to implement pb.KeyServiceServer
type server struct {
	pb.UnimplementedKeyServiceServer
	// need to include the store variable later
}

// Function for registering key, take in the RegisterKeyRequest from context
func (s *server) RegisterKey(ctx context.Context, in *pb.RegisterKeyRequest) (*pb.RegisterKeyResponse, error) {
	// Store the key after registration and replicate the key to other servers?
	
	return &pb.RegisterKeyResponse{
		Success: true,
		Message: "Key successfully registered with TTL",
	}, nil
}

// GetKey function to implement retrieval of key from server for Rust node to verify
func (s *server) GetKey(ctx context.Context in *pb.GetKeyRequest) (*pb.RegisterKeyResponse, error) {
	// Get the Key struct from the stores
	// If the ttl is valid, return the key to the client;
	// If ttl expired, return an error response to the client
	return &pb.GetKeyResponse{
		Success: true,
	}, nil
}