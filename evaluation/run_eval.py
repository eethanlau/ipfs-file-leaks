import os
import subprocess
import time
import json
import logging

logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')

SIZES = {"1KB": 1024, "1MB": 1024**2, "10MB": 10 * 1024**2, "50MB": 50 * 1024**2}
DATA_DIR = "data"
RESULTS_FILE = "results/report.json"

# Paths resolved from the script's own location so the script can be run from anywhere.
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
ENCRYPTION_NODE_DIR = os.path.abspath(os.path.join(SCRIPT_DIR, "..", "encryption-node"))
ENCRYPTION_BIN = os.path.join(ENCRYPTION_NODE_DIR, "target", "release", "encryption-node")

def run_cmd(cmd, env=None, check=True, timeout=60):
    logging.info(f"Running: {cmd}")
    try:
        result = subprocess.run(cmd, shell=True, env=env, text=True, capture_output=True, timeout=timeout)
        if check and result.returncode != 0:
            logging.error(f"Command failed: {result.stderr}")
            raise Exception(f"Command failed: {cmd}\\nOutput: {result.stderr}")
        return result.stdout.strip()
    except subprocess.TimeoutExpired:
        logging.error(f"Command timed out after {timeout}s: {cmd}")
        raise Exception(f"Command timed out: {cmd}")

def setup_data():
    os.makedirs(DATA_DIR, exist_ok=True)
    for name, size in SIZES.items():
        path = os.path.join(DATA_DIR, f"file_{name}.bin")
        if not os.path.exists(path):
            logging.info(f"Generating {path} of size {size} bytes")
            with open(path, "wb") as f:
                f.write(os.urandom(size))

def build_encryption_node():
    logging.info(f"Building encryption-node release binary at {ENCRYPTION_BIN}...")
    run_cmd(f"cargo build --release --bin encryption-node --manifest-path {ENCRYPTION_NODE_DIR}/Cargo.toml", timeout=600)
    if not os.path.exists(ENCRYPTION_BIN):
        raise Exception(f"Release binary not found after build: {ENCRYPTION_BIN}")

def setup_testbed():
    logging.info("Starting testbed...")
    run_cmd("docker compose -f docker-compose.yml down -v", check=False)
    run_cmd("docker compose -f docker-compose.yml up --build -d")
    
    logging.info("Waiting for IPFS nodes to be ready (15s)...")
    time.sleep(15)

    # Strip public bootstrap peers and restart so the daemons come back up
    # isolated. Without this, each Kubo node connects to ~100 public IPFS
    # peers; bitswap WANTs against the test peers get drowned out and the
    # first cross-container retrieval times out.
    nodes = [
        "ipfs-publisher",
        "ipfs-retriever-fast",
        "ipfs-retriever-slow",
        "ipfs-retriever-third-party",
    ]
    for node in nodes:
        run_cmd(f"docker exec {node} ipfs bootstrap rm --all", check=False)
    logging.info("Restarting IPFS containers to drop public peer connections...")
    run_cmd("docker compose -f docker-compose.yml restart " + " ".join(nodes))
    time.sleep(15)

    # Get publisher ID
    pub_id_json = run_cmd("docker exec ipfs-publisher ipfs id")
    pub_id_data = json.loads(pub_id_json)
    pub_peer_id = pub_id_data.get("ID")
    pub_multiaddr = f"/dns4/ipfs-publisher/tcp/4001/p2p/{pub_peer_id}"
    logging.info(f"Publisher Multiaddr: {pub_multiaddr}")

    # Connect standard retrievers to publisher
    run_cmd(f"docker exec ipfs-retriever-fast ipfs swarm connect {pub_multiaddr}", check=False)

    # Connect third-party ONLY to the fast retriever
    fast_id_json = run_cmd("docker exec ipfs-retriever-fast ipfs id")
    fast_id_data = json.loads(fast_id_json)
    fast_peer_id = fast_id_data.get("ID")
    fast_multiaddr = f"/dns4/ipfs-retriever-fast/tcp/4001/p2p/{fast_peer_id}"
    run_cmd(f"docker exec ipfs-retriever-third-party ipfs swarm connect {fast_multiaddr}", check=False)

    # Warmup probe: publish a tiny file from publisher and confirm the fast
    # retriever can fetch it. Catches cold-bitswap failures up front rather
    # than silently timing out the first measured retrieve. Retried because
    # bitswap convergence after a fresh swarm connect is non-deterministic.
    logging.info("Warming up bitswap session between publisher and fast retriever...")
    warmup_cid = run_cmd(
        "echo warmup-$(date +%s) | docker exec -i ipfs-publisher ipfs add -q --pin=false"
    ).strip()
    last_err = None
    for attempt in range(1, 7):
        try:
            run_cmd(
                f"docker exec ipfs-retriever-fast ipfs cat --timeout=10s {warmup_cid}",
                timeout=15,
            )
            logging.info(f"Warmup complete on attempt {attempt}; bitswap is live.")
            break
        except Exception as e:
            last_err = e
            logging.warning(f"Warmup attempt {attempt} failed; retrying...")
            time.sleep(3)
    else:
        raise Exception(f"Warmup never converged after 6 attempts: {last_err}")

    # Inject latency to slow node
    try:
        run_cmd("docker exec -u 0 ipfs-retriever-slow apk add iproute2", check=False)
        # run_cmd("docker exec -u 0 ipfs-retriever-slow tc qdisc add dev eth0 root netem delay 200ms", check=False)
        # logging.info("Injected 200ms latency to ipfs-retriever-slow")
    except Exception as e:
        logging.warning("Could not inject latency, continuing without it.")

def run_evaluation():
    os.makedirs("results", exist_ok=True)
    results = {}

    for name, size in SIZES.items():
        logging.info(f"\\n{'='*40}\\n=== Evaluating {name} ===\\n{'='*40}")
        file_path = os.path.abspath(os.path.join(DATA_DIR, f"file_{name}.bin"))

        results[name] = {"size_bytes": size}

        # ------------------------------------------
        # 1. Base Leak & Third Party Viral Leak Test
        # ------------------------------------------
        env = os.environ.copy()
        env["IPFS_URL"] = "http://127.0.0.1:5031"
        env["KEY_SERVER_URL"] = "http://127.0.0.1:50061"

        # Publish
        start_time = time.time()
        output = run_cmd(f"{ENCRYPTION_BIN} publish \"{file_path}\"", env=env)
        pub_time = time.time() - start_time
        cid = [p.split("=")[1] for line in output.split('\\n') for p in line.split() if p.startswith("cid=")][0]
        logging.info(f"Published {name}, CID: {cid}, Time: {pub_time:.2f}s")

        # Retrieve Fast
        env["IPFS_URL"] = "http://127.0.0.1:5041"
        start_time = time.time()
        run_cmd(f"{ENCRYPTION_BIN} retrieve {cid}", env=env)
        fast_time = time.time() - start_time
        logging.info(f"Retrieved {name} (fast), Time: {fast_time:.2f}s")
        
        # Remove from Publisher
        logging.info(f"Removing {cid} from publisher...")
        run_cmd(f"docker exec ipfs-publisher ipfs pin rm {cid}", check=False)
        run_cmd("docker exec ipfs-publisher ipfs repo gc")
        time.sleep(5)
        
        # Check if leak exists
        leak_exists = False
        try:
            run_cmd(f"docker exec ipfs-retriever-fast ipfs block stat {cid}")
            leak_exists = True
        except Exception:
            pass
        logging.info(f"Leak exists for {name}? {leak_exists}")
        
        # Third Party Viral Leak Test
        third_party_leaked = False
        try:
            logging.info(f"Checking if third-party can fetch the leaked {name} data...")
            result = subprocess.run(f"docker exec ipfs-retriever-third-party ipfs cat --timeout=60s {cid} > /dev/null", shell=True, stderr=subprocess.PIPE)
            if result.returncode == 0:
                third_party_leaked = True
        except Exception:
            pass
        logging.info(f"Third-party viral leak for {name}? {third_party_leaked}")
        
        results[name].update({
            "publish_time_s": round(pub_time, 3),
            "retrieve_fast_time_s": round(fast_time, 3),
            "leak_exists_after_publisher_delete": leak_exists,
            "third_party_viral_leak": third_party_leaked
        })

        # ------------------------------------------
        # 2. TTL Expiration Mitigation Test (See if can decrypt once the TTL expires)
        # ------------------------------------------
        logging.info(f"--- Running TTL Expiration Test for {name} ---")
        env["IPFS_URL"] = "http://127.0.0.1:5031"
        output = run_cmd(f"{ENCRYPTION_BIN} --ttl 15s publish \"{file_path}\"", env=env)
        ttl_cid = [p.split("=")[1] for line in output.split('\\n') for p in line.split() if p.startswith("cid=")][0]

        logging.info("Retrieving immediately (should succeed)...")
        env["IPFS_URL"] = "http://127.0.0.1:5041"
        run_cmd(f"{ENCRYPTION_BIN} retrieve {ttl_cid}", env=env)

        logging.info("Waiting 16 seconds for TTL to expire...")
        time.sleep(16)

        logging.info("Retrieving after TTL (should fail)...")
        ttl_mitigation_worked = False
        try:
            run_cmd(f"{ENCRYPTION_BIN} retrieve {ttl_cid}", env=env)
        except Exception as e:
            logging.info("Retrieval failed as expected! Mitigation works.")
            ttl_mitigation_worked = True
            
        results[name]["ttl_mitigation_successful"] = ttl_mitigation_worked
        
        # ------------------------------------------
        # 3. Encryption Overhead Benchmark
        # ------------------------------------------
        logging.info(f"--- Running Encryption Overhead Benchmark for {name} ---")
        
        # Native IPFS add
        start_time = time.time()
        add_out = run_cmd(f"curl -s -F file=@\"{file_path}\" http://127.0.0.1:5031/api/v0/add")
        native_pub_time = time.time() - start_time
        native_cid = json.loads(add_out)["Hash"]
        
        # Native IPFS cat
        start_time = time.time()
        run_cmd(f"curl -s -o /dev/null http://127.0.0.1:5031/api/v0/cat?arg={native_cid}")
        native_ret_time = time.time() - start_time
        
        results[name].update({
            "overhead_benchmark": {
                "native_publish_time_s": round(native_pub_time, 3),
                "native_retrieve_time_s": round(native_ret_time, 3),
                "publish_overhead_percent": round(((pub_time / native_pub_time) - 1) * 100, 2),
                "retrieve_overhead_percent": round(((fast_time / native_ret_time) - 1) * 100, 2),
            }
        })

        with open(RESULTS_FILE, "w") as f:
            json.dump(results, f, indent=2)
            
    logging.info(f"Saved results to {RESULTS_FILE}")

def run_fault_tolerance_test():
    """Publish through the primary key server, kill it, retrieve via the
    replica. Confirms that replication populates the secondary so retrievals
    survive a single-server failure."""
    logging.info(f"\\n{'='*40}\\n=== Fault Tolerance Test ===\\n{'='*40}")
    file_path = os.path.abspath(os.path.join(DATA_DIR, "file_1KB.bin"))

    env = os.environ.copy()
    env["IPFS_URL"] = "http://127.0.0.1:5031"
    env["KEY_SERVER_URL"] = "http://127.0.0.1:50061"

    output = run_cmd(f"{ENCRYPTION_BIN} publish \"{file_path}\"", env=env)
    cid = [p.split("=")[1] for line in output.split('\\n') for p in line.split() if p.startswith("cid=")][0]
    logging.info(f"Published via primary, CID: {cid}")

    # Allow the asynchronous replication broadcast to land on the replica.
    time.sleep(2)

    logging.info("Killing primary key-server...")
    run_cmd("docker kill eval-key-server")

    env["IPFS_URL"] = "http://127.0.0.1:5041"
    env["KEY_SERVER_URL"] = "http://127.0.0.1:50062"
    fault_tolerant = False
    try:
        run_cmd(f"{ENCRYPTION_BIN} retrieve {cid}", env=env)
        fault_tolerant = True
        logging.info("Retrieval succeeded with primary down — replication holds.")
    except Exception:
        logging.error("Retrieval failed with primary down — replica did not have the key.")

    logging.info("Restarting primary key-server...")
    run_cmd("docker start eval-key-server", check=False)

    with open(RESULTS_FILE, "r") as f:
        results = json.load(f)
    results["fault_tolerance"] = {"primary_killed_then_retrieved_via_replica": fault_tolerant}
    with open(RESULTS_FILE, "w") as f:
        json.dump(results, f, indent=2)
    logging.info(f"Fault tolerance result: {fault_tolerant}")

if __name__ == "__main__":
    setup_data()
    build_encryption_node()
    setup_testbed()
    try:
        run_evaluation()
        run_fault_tolerance_test()
    finally:
        logging.info("Cleaning up testbed...")
        run_cmd("docker compose -f docker-compose.yml down")
