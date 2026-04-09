package store

import (
	"errors"
	"sync"
	"time"
)

// KeyRecord holds the symmetric key and its absolute expiration time
type KeyRecord struct {
	EncryptionKey []byte
	ExpiresAt     time.Time
}

// InMemoryStore for thread-safe map to hold keys
type InMemoryStore struct {
	mu   sync.RWMutex
	keys map[string]KeyRecord
}

// NewInMemoryStore initializes key store
func NewInMemoryStore() *InMemoryStore {
	return &InMemoryStore{
		keys: make(map[string]KeyRecord),
	}
}

// Set saves a key with a specific TTL
func (s *InMemoryStore) Set(cid string, key []byte, ttlSeconds int64) {
	s.mu.Lock()
	defer s.mu.Unlock()

	// Calculate the exact time this key should die
	expiresAt := time.Now().Add(time.Duration(ttlSeconds) * time.Second)

	s.keys[cid] = KeyRecord{
		EncryptionKey: key,
		ExpiresAt:     expiresAt,
	}
}

// Get retrieves a key, but enforces the TTL check first
func (s *InMemoryStore) Get(cid string) ([]byte, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()

	record, exists := s.keys[cid]
	if !exists {
		return nil, errors.New("key not found")
	}

	// This is where we enforce the expiration your paper talks about
	if time.Now().After(record.ExpiresAt) {
		// Optional: you could actively delete the key from the map here to save memory
		return nil, errors.New("key has expired")
	}

	return record.EncryptionKey, nil
}
