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

// NewInMemoryStore initializes key store struct
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
	record, exists := s.keys[cid]
	s.mu.RUnlock() // Release read lock before another write operation

	if !exists {
		return nil, errors.New("key not found")
	}

	// Enforce expiration and upgrade to write lock for delete operation if past the ttl
	if time.Now().After(record.ExpiresAt) {
		s.mu.Lock()
		delete(s.keys, cid)
		s.mu.Unlock()
		return nil, errors.New("key has expired")
	}

	return record.EncryptionKey, nil
}
