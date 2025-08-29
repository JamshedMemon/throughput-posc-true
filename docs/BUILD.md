# Throughput PoSC Blockchain - Build Instructions

## Overview
This project implements a custom Proof of Schedule (PoSC) consensus mechanism based on the Throughput whitepaper algorithms, with full Ethereum/EVM compatibility via Frontier.

## Prerequisites

### macOS with Colima (Recommended)
- Docker Desktop or Colima installed
- For Colima: `brew install colima docker`
- Start Colima: `colima start --cpu 4 --memory 8 --disk 100`

### Linux/Windows
- Docker installed and running
- At least 8GB RAM and 60GB free disk space

## Building the Docker Image

### 1. Clean Build
```bash
# Navigate to project directory
cd throughput-posc-true

# Build the Docker image
docker build -t throughput-substrate .
```

### 2. Rebuild After Code Changes
```bash
# Remove the old image (optional, for clean rebuild)
docker rmi throughput-substrate

# Rebuild with no cache to ensure all changes are included
docker build --no-cache -t throughput-substrate .
```

### 3. Build with Custom Tag/Version
```bash
docker build -t throughput-substrate:v1.0.0 .
```

## Important Files

### Consensus Implementation
- `pallets/posc/src/lib.rs` - Main PoSC pallet implementation
- `primitives/posc/src/eip.rs` - Elastic Initiation Proposal (Algorithm 1)
- `primitives/posc/src/eabs.rs` - Elastic Advanced Block Schedule (Algorithm 3)
- `consensus/posc/` - Consensus engine integration

### EVM/Frontier Components
- `frame/ethereum/` - Ethereum compatibility layer
- `frame/evm/` - EVM execution engine
- `client/rpc/` - Ethereum RPC endpoints

### Configuration Files
- `Dockerfile` - Docker build configuration
- `Cargo.toml` - Rust workspace configuration
- `.dockerignore` - Files to exclude from Docker context

## Troubleshooting Build Issues

### 1. Disk Space Issues
```bash
# Check available space
df -h .

# Clean Docker cache
docker system prune -a

# Remove unused volumes
docker volume prune
```

### 2. Memory Issues
```bash
# For Colima, increase memory allocation
colima stop
colima start --cpu 4 --memory 12 --disk 100
```

### 3. Build Failures Due to Environment Variables
The Dockerfile already handles this, but if you encounter issues:
- Ensure `.cargo/config.toml` is excluded via `.dockerignore`
- The Dockerfile unsets macOS-specific environment variables

### 4. Network Issues During Build
```bash
# Build with increased network timeout
docker build --network-timeout 300 -t throughput-substrate .
```

## Running the Node

### Development Mode
```bash
docker run -p 9944:9944 -p 9933:9933 -p 30333:30333 throughput-substrate --dev
```

### With Persistent Data
```bash
docker run -p 9944:9944 -p 9933:9933 -p 30333:30333 \
  -v throughput-data:/data \
  throughput-substrate --dev --base-path /data
```

### Custom Chain Configuration
```bash
docker run -p 9944:9944 -p 9933:9933 -p 30333:30333 \
  throughput-substrate \
  --chain custom-spec.json \
  --validator
```

## Verifying the Build

### Check Image Details
```bash
docker images | grep throughput-substrate
```

### Inspect Image Layers
```bash
docker history throughput-substrate
```

### Test Basic Functionality
```bash
# Run temporarily to check if it starts
docker run --rm throughput-substrate --version
```

## Development Workflow

### 1. Make Code Changes
Edit the relevant files in your local directory

### 2. Rebuild Docker Image
```bash
docker build -t throughput-substrate:dev .
```

### 3. Test Changes
```bash
docker run -p 9944:9944 throughput-substrate:dev --dev --tmp
```

### 4. Connect to Node
- WebSocket: `ws://localhost:9944`
- HTTP RPC: `http://localhost:9933`
- P2P: `localhost:30333`

## Custom Build Arguments

### Build with Specific Rust Version
```dockerfile
# Edit Dockerfile first line:
FROM rust:1.76-bullseye as builder
```

### Build Specific Binary
```dockerfile
# Edit Dockerfile build command:
RUN cargo build --release --bin your-binary-name
```

## Build Time Estimates
- First build: 15-20 minutes (downloading dependencies)
- Subsequent builds: 5-10 minutes (with cache)
- Clean rebuild: 15-20 minutes

## Monitoring Build Progress
```bash
# Watch build output
docker build -t throughput-substrate . 2>&1 | tee build.log

# Check build status in another terminal
docker ps -a
```

## CI/CD Integration

### GitHub Actions Example
```yaml
name: Build Docker Image
on: [push]
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v2
    - name: Build Docker image
      run: docker build -t throughput-substrate:${{ github.sha }} .
```

## Notes
- The build process compiles the Rust code inside the Docker container, ensuring consistency across different host systems
- All builds use the Debian Bullseye base image for the runtime
- The Rust toolchain is automatically managed within the Docker build

## Support
For issues or questions about the build process, check:
- `BUILD_STATUS.md` - Current build status
- `COMPILATION_STATUS.md` - Compilation details
- GitHub Issues - For bug reports