package server

import (
	"context"

	pb "ipfs-file-leaks/key-server/pb"
	"ipfs-file-leaks/key-server/store"
)

// Server implements pb.KeyServiceServer
type Server struct {
	pb.UnimplementedKeyServiceServer
	store *store.InMemoryStore
}

// NewServer creates a Server with an initialized in-memory key store
func NewServer() *Server {
	return &Server{
		store: store.NewInMemoryStore(),
	}
}

// RegisterKey stores the encryption key for a CID with the given TTL
func (s *Server) RegisterKey(ctx context.Context, in *pb.RegisterKeyRequest) (*pb.RegisterKeyResponse, error) {
	s.store.Set(in.Cid, in.EncryptionKey, in.TtlSeconds)

	return &pb.RegisterKeyResponse{
		Success: true,
		Message: "Key registered successfully",
	}, nil
}

// GetKey retrieves the encryption key for a CID if its TTL has not expired
func (s *Server) GetKey(ctx context.Context, in *pb.GetKeyRequest) (*pb.GetKeyResponse, error) {
	key, err := s.store.Get(in.Cid)
	if err != nil {
		// Key not found or TTL expired — return a failed response, not a Go error
		return &pb.GetKeyResponse{
			Success: false,
			Message: err.Error(),
		}, nil
	}

	return &pb.GetKeyResponse{
		Success:       true,
		EncryptionKey: key,
	}, nil
}