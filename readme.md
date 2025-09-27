# Clippr

A secure Solana wallet with Multi-Party Computation (MPC) for institutional-grade security and real-time blockchain monitoring.

## Features

- **MPC Security**: Threshold signatures for enhanced private key protection
- **Jupiter Integration**: Seamless token swaps with best price routing
- **Real-time Monitoring**: Live balance updates and transaction tracking
- **Institutional Grade**: Enterprise-level security and reliability

## Architecture

- **Backend**: Actix-web API server with comprehensive wallet operations
- **Indexer**: Real-time Solana blockchain monitoring service
- **MPC Server**: Distributed key management and threshold signatures
- **Database**: PostgreSQL with optimized schemas for performance

## Quick Start

### Prerequisites
- Rust 1.70+
- PostgreSQL 13+
- Solana CLI tools

### Run Services

```bash
# Start backend server
cargo run --bin backend

# Start indexer service
cargo run --bin indexer

# Start MPC server
cargo run --bin mpc-simple
```

## API Endpoints

- **Health**: `GET /api/v1/health`
- **Balance**: `GET /api/v1/balance/{pubkey}`
- **Swap**: `POST /api/v1/jupiter/swap`
- **Subscribe**: `POST /api/v1/keys/subscribe`

## Configuration

Services use environment variables for configuration:
- `DATABASE_URL`: PostgreSQL connection string
- `SOLANA_RPC_URL`: Solana RPC endpoint
- `YELLOWSTONE_ENDPOINT`: Geyser streaming endpoint

## Security

- Multi-party threshold signatures
- Secure key generation and storage
- Real-time transaction monitoring
- Enterprise-grade access controls
