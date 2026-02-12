# Fiber Demo Startup

This repository provides a complete local development/testing environment for [Fiber Network](https://github.com/nervosnetwork/fiber), using Docker Compose to launch a CKB development chain and multiple Fiber nodes with a single command.

## Purpose

This is a Fiber Network demo environment designed for:

- Local testing of Fiber Network payment channel functionality
- Developing and debugging Fiber-based applications
- Learning and understanding how Fiber Network works
- Testing Lightning Network-style payments with CKB native tokens and sUDT tokens

## Quick Start

### Prerequisites

- Docker
- Docker Compose

### Launch Services

```bash
docker compose up --build
```

The first build requires compiling CKB and Fiber from source, which may take a considerable amount of time.

### Clean Up Environment

To reset the Fiber nodes' state (e.g., channels, payment history), stop the services and delete the store directories:

```bash
docker compose down
rm -rf fiber/nodes/*/store
```

Then restart with `docker compose up` to start fresh.

### Service Ports

| Service | RPC Port | P2P Port |
|---------|----------|----------|
| CKB | 8114 | - |
| fiber-bootnode | 8230 | 10000 |
| fiber-node1 | 8231 | 10001 |
| fiber-node2 | 8232 | 10002 |
| fiber-node3 | 8233 | 10003 |
| fiber-web | 3000 | - |

### Calling Fiber RPC

```bash
# Query node info
curl -X POST http://127.0.0.1:8231 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"node_info","params":[],"id":1}'
```

## Docker Images

This project contains 7 Docker images:

### 1. ckb

**Purpose**: Runs the CKB development chain node

- Based on Debian 12, compiles CKB from source
- Configured in development mode with built-in miner
- Pre-deployed with smart contracts required by Fiber (FundingLock, CommitmentLock, simple_udt, etc.)
- sUDT tokens are pre-minted in the genesis block, owned by a pre-configured source account
- Exposes RPC port 8114

### 2. fiber-bootnode

**Purpose**: Bootstrap node for the Fiber network

- Other Fiber nodes discover peers through this node
- Runs the Fiber daemon (fnn)
- Configured as the entry point for the gossip network
- RPC port: 8230, P2P port: 10000

### 3. fiber-node1 / fiber-node2 / fiber-node3

**Purpose**: Regular Fiber nodes

- Three independent Fiber nodes for testing payment channels
- Automatically connect to bootnode on startup
- Each node has independent keys and wallet
- Can establish payment channels between nodes and send CKB and sUDT payments

### 4. transfer

**Purpose**: Initial fund distribution tool

- One-time container that runs and exits
- Since sUDT tokens are pre-minted in the genesis block (owned by a source account), this tool transfers both CKB and sUDT from the source account to each Fiber node for testing purposes
- Transfers 1 billion CKB to each node (bootnode, node1, node2, node3)
- Transfers 1 billion sUDT to node1, node2, and node3
- After distribution, each Fiber node has sufficient funds to open payment channels and perform test transactions

### 5. fiber-web

**Purpose**: Web-based monitoring and management panel ([fiber-nodes-monit](https://github.com/gpBlockchain/fiber-nodes-monit))

- Provides a web UI for monitoring and operating Fiber nodes
- Built with React + TypeScript + Vite, served by a Node.js backend
- Includes a JSON-RPC proxy that forwards browser requests to Fiber node RPC endpoints
- Accessible at http://127.0.0.1:3000 after startup
- To add nodes for monitoring, use the Docker internal service names as RPC URLs:
  - `http://fiber-bootnode:10000`
  - `http://fiber-node1:10000`
  - `http://fiber-node2:10000`
  - `http://fiber-node3:10000`

## Directory Structure

```
.
├── docker-compose.yml      # Docker Compose configuration
├── ckb/                    # CKB node configuration
│   ├── Dockerfile          # CKB image build file
│   ├── dev.toml            # CKB dev chain configuration
│   ├── contracts/          # Pre-deployed smart contracts
│   └── run.sh              # CKB startup script
├── fiber/                  # Fiber node configuration
│   ├── Dockerfile          # Fiber image build file (generic, shared by all nodes)
│   ├── Dockerfile.transfer # Transfer tool image build file
│   ├── contracts/          # Fiber contracts
│   ├── start.sh            # Fiber node startup script
│   ├── transfer/           # Fund distribution tool source code
│   └── nodes/              # Per-node configuration directories
│       ├── bootnode/       # Bootnode configuration
│       │   ├── ckb/
│       │   │   └── key     # CKB account private key
│       │   ├── config.yml  # Node configuration
│       │   ├── dev.toml    # Chain spec
│       │   ├── fiber/
│       │   │   └── sk      # Fiber node secret key
│       │   └── store/      # Runtime data (created automatically, delete to reset)
│       ├── node1/          # Node1 configuration (same structure)
│       ├── node2/          # Node2 configuration (same structure)
│       └── node3/          # Node3 configuration (same structure)
├── fiber-web/              # Web monitoring panel
│   └── Dockerfile          # fiber-nodes-monit image build file
```

Each node directory follows a standardized layout and is mounted into the container at runtime via Docker volumes. The `store/` directory is created automatically when the node runs and contains the node's state data.

## Startup Order

Docker Compose starts services in the following order:

1. **ckb** - Starts the CKB development chain first
2. **transfer** - Runs fund distribution after CKB is ready
3. **fiber-bootnode** - Starts the bootstrap node after CKB is ready
4. **fiber-node1/2/3** - Start regular nodes after bootnode is ready
5. **fiber-web** - Starts the web monitoring panel after bootnode is ready

## Notes

- All data is ephemeral and will be lost when containers restart; suitable for development and testing
- Private keys are for testing purposes only; do not use in production
- First-time image build takes a long time; please be patient
