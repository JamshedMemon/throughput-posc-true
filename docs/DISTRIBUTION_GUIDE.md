# Throughput PoSC Node Distribution Guide

## What to Share

You have **two options** for distributing your Throughput PoSC node:

### Option 1: Docker Image (Recommended) - 153MB
The Docker image is the cleanest and most portable way to share your node.

#### Export the Docker Image:
```bash
# Export to a tar file (153MB compressed)
docker save throughput-substrate:latest | gzip > throughput-posc-node.tar.gz

# The file size will be approximately 153MB
ls -lh throughput-posc-node.tar.gz
```

#### Share via Cloud Storage:
Upload `throughput-posc-node.tar.gz` to:
- Google Drive
- Dropbox  
- WeTransfer
- GitHub Releases (if public)

#### Recipients Import & Run:
```bash
# Load the image
docker load < throughput-posc-node.tar.gz

# Run the node
docker run -p 9944:9944 -p 9933:9933 -p 30333:30333 \
  throughput-substrate /usr/local/bin/frontier-template-node --dev
```

### Option 2: Source Code + Dockerfile
If they want to build from source, share these files:

```bash
# Create distribution archive
tar -czf throughput-posc-source.tar.gz \
  --exclude=target \
  --exclude=.git \
  --exclude=node_modules \
  --exclude=*.log \
  --exclude=frontier-fresh \
  --exclude=substrate-minimal \
  Cargo.toml \
  Cargo.lock \
  Dockerfile \
  .dockerignore \
  pallets/ \
  primitives/ \
  client/ \
  consensus/ \
  frame/ \
  template/ \
  scripts/ \
  precompiles/
```

### Option 3: Docker Hub (Best for Public Distribution)
```bash
# Tag your image
docker tag throughput-substrate:latest yourusername/throughput-posc:latest

# Push to Docker Hub
docker push yourusername/throughput-posc:latest

# Recipients can simply run:
docker run -p 9944:9944 yourusername/throughput-posc:latest \
  /usr/local/bin/frontier-template-node --dev
```

## Quick Start Instructions for Recipients

### Requirements
- Docker installed
- 8GB RAM minimum
- 10GB free disk space

### Running the Node

#### Development Mode (Single Node)
```bash
docker run -d --name throughput-node \
  -p 9944:9944 -p 9933:9933 -p 30333:30333 \
  throughput-substrate /usr/local/bin/frontier-template-node \
  --dev --tmp
```

#### Local Testnet (Multiple Validators)
```bash
# Node 1 - Alice
docker run -d --name alice \
  -p 30333:30333 -p 9944:9944 \
  throughput-substrate /usr/local/bin/frontier-template-node \
  --alice --validator --chain local

# Node 2 - Bob (on same machine, different ports)
docker run -d --name bob \
  -p 30334:30334 -p 9945:9945 \
  throughput-substrate /usr/local/bin/frontier-template-node \
  --bob --validator --chain local \
  --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/[ALICE_PEER_ID]
```

### Connecting to the Node
- **Polkadot.js Apps**: https://polkadot.js.org/apps/?rpc=ws://127.0.0.1:9944
- **WebSocket RPC**: ws://localhost:9944
- **HTTP RPC**: http://localhost:9933

### Important Files in the Image
- Binary: `/usr/local/bin/frontier-template-node`
- Chain specs: Built into the binary
- PoSC consensus: Integrated in the runtime

## Minimal Distribution Package

If you want the absolute minimum, just share:
1. The Docker image tar.gz file (153MB)
2. This README with run instructions

That's all anyone needs to run your Throughput PoSC node!

## Support Documentation

Include these documents if sharing source:
- `Throughput.pdf` - Whitepaper explaining PoSC algorithms
- `BUILD_README.md` - Build instructions
- `CONSENSUS_FIXES.md` - Implementation details

## License & Attribution
[Add your license information here]

---
Built with Substrate and Frontier for EVM compatibility.
PoSC (Proof of Schedule) consensus implementation.