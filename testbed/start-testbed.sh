#!/usr/bin/env bash
# start-testbed.sh — bring up the full IPFS + key-server testbed
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "==> Starting testbed (IPFS + Key Server)..."
docker compose -f "$SCRIPT_DIR/docker-compose.yml" up --build -d

echo "==> Waiting for IPFS node to be healthy..."
until docker exec ipfs-node ipfs id > /dev/null 2>&1; do
  sleep 1
done

echo ""
echo "   Testbed is ready"
echo "   IPFS HTTP API : http://127.0.0.1:5001"
echo "   IPFS Gateway  : http://127.0.0.1:8080"
echo "   Key Server    : grpc://127.0.0.1:50051"
echo ""
echo "To run a baseline publish test:"
echo "   cd ../encryption-node && cargo run -- publish <your_file>"
echo ""
echo "To stop the testbed:"
echo "   docker compose -f $SCRIPT_DIR/docker-compose.yml down"
