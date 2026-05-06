package server

import (
	"context"
	"log"

	pb "ipfs-file-leaks/key-server/pb"
	"ipfs-file-leaks/key-server/replication"
	"ipfs-file-leaks/key-server/store"
)

// Server implements pb.KeyServiceServer
type Server struct {
	pb.UnimplementedKeyServiceServer
	store      *store.InMemoryStore
	replicator *replication.Replicator
}

// NewServer creates a Server with an initialized in-memory key store and a
// replicator that forwards new registrations to the supplied peer addresses.
// Pass an empty slice to run as a single, non-replicating server.
func NewServer(peers []string) *Server {
	return &Server{
		store:      store.NewInMemoryStore(),
		replicator: replication.NewReplicator(peers),
	}
}

// RegisterKey stores the encryption key for a CID with the given TTL and,
// when this is an original write (IsReplication=false), broadcasts it to
// peer servers so reads can be served by any replica.
func (s *Server) RegisterKey(ctx context.Context, in *pb.RegisterKeyRequest) (*pb.RegisterKeyResponse, error) {
	s.store.Set(in.Cid, in.EncryptionKey, in.TtlSeconds)

	if in.IsReplication {
		log.Printf("Stored replicated key for CID %s", in.Cid)
	} else {
		s.replicator.BroadcastKey(in)
	}

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
