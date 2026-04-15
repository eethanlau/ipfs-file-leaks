package replication

import (
	"context"
	"log"

	pb "ipfs-file-leaks/key-server/pb"

	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

// Replicator handles forwarding keys to other Go Key Servers
type Replicator struct {
	peers []string // Example: []string{"server2:50051", "server3:50051"}
}

// NewReplicator initializes the replicator with a list of known peer addresses
func NewReplicator(peers []string) *Replicator {
	return &Replicator{
		peers: peers,
	}
}

// BroadcastKey sends the newly registered key to all known peers
func (r *Replicator) BroadcastKey(req *pb.RegisterKeyRequest) {
	// 1. Create a copy of the request and flip the replication flag so peers don't re-broadcast it endlessly!
	replicationReq := &pb.RegisterKeyRequest{
		Cid:           req.Cid,
		EncryptionKey: req.EncryptionKey,
		TtlSeconds:    req.TtlSeconds,
		IsReplication: true,
	}

	for _, peerAddr := range r.peers {
		// 2. Launch a goroutine for each peer so we don't slow down the main server response
		go func(addr string) {
			
			// 3. Connect to the peer server over gRPC (acting as a client now)
			conn, err := grpc.Dial(addr, grpc.WithTransportCredentials(insecure.NewCredentials()))
			if err != nil {
				log.Printf("Failed to connect to peer %s: %v", addr, err)
				return
			}
			defer conn.Close()

			client := pb.NewKeyServiceClient(conn)
			
			// 4. Send the key forward
			_, err = client.RegisterKey(context.Background(), replicationReq)
			if err != nil {
				log.Printf("Failed to replicate key to peer %s: %v", addr, err)
			} else {
				log.Printf("Successfully replicated key to peer %s", addr)
			}
			
		}(peerAddr)
	}
}
