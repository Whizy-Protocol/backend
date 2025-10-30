# Whizy Prediction Market Backend

A high-performance Rust backend server for the Whizy prediction market platform built on the HEDERA Testnet. This server provides comprehensive APIs for market management, betting operations, yield generation, and blockchain synchronization.

## Overview

Whizy is a decentralized prediction market platform that combines traditional prediction markets with DeFi yield farming strategies. Users can create markets, place bets, and earn yield on their deposits through integrated protocols like Aave, Compound, and Morpho.

## Features

- **Prediction Markets**: Create and manage prediction markets with automated market maker (AMM) functionality
- **Betting System**: Place bets on market outcomes with dynamic odds calculation
- **Yield Generation**: Earn yield on deposited funds through integrated DeFi protocols
- **Blockchain Integration**: Full blockchain synchronization with HEDERA Testnet
- **Real-time APIs**: RESTful APIs with comprehensive market and user data
- **Admin Panel**: Administrative tools for market and protocol management
- **Authentication**: JWT-based authentication system
- **OpenAPI Documentation**: Auto-generated API documentation with Swagger UI

## Architecture

### Core Components

- **Market Service**: Handles market creation, management, and statistics
- **Bet Service**: Manages betting operations and odds calculations
- **Protocol Service**: Integrates with DeFi protocols for yield generation
- **Blockchain Sync Service**: Synchronizes on-chain events with the database
- **User Service**: Manages user accounts and statistics
- **Stats Service**: Provides platform-wide analytics

### Supported Protocols

- **Aave**: Lending protocol integration
- **Compound**: Money market protocol
- **Morpho**: Optimized lending protocol

## Quick Start

### Prerequisites

- Rust 1.70+ with Cargo
- PostgreSQL 14+
- HEDERA Testnet access

### Installation

1. **Clone the repository**
   ```bash
   git clone <repository-url>
   cd backend
   ```

2. **Install dependencies**
   ```bash
   cargo build --release
   ```

3. **Setup environment variables**
   ```bash
   cp .env.example .env
   # Edit .env with your configuration
   ```

4. **Setup database**
   ```bash
   # Create PostgreSQL database
   createdb whizy_testnet
   
   # Run migrations
   sqlx migrate run
   ```

5. **Start the server**
   ```bash
   cargo run --release
   ```

The server will start on `http://localhost:3002` by default.

## Configuration

### Environment Variables

#### Database
- `DATABASE_URL`: PostgreSQL connection string
- `DATABASE_TIMEZONE`: Database timezone (default: UTC)

#### Server
- `HOST`: Server host (default: 0.0.0.0)
- `PORT`: Server port (default: 3002)
- `RUST_LOG`: Logging level (default: info)
- `CORS_ORIGIN`: CORS origin (default: *)

#### Authentication
- `API_KEY`: API key for protected endpoints
- `JWT_SECRET`: Secret for JWT token signing

#### HEDERA Testnet
- `HEDERA_RPC_URL`: HEDERA RPC endpoint
- `HEDERA_CHAIN_ID`: Chain ID (2484 for testnet)
- `PRIVATE_KEY`: Private key for blockchain operations

#### Contract Addresses
- `WHIZY_PREDICTION_MARKET_ADDR`: Main prediction market contract
- `PROTOCOL_SELECTOR_ADDR`: Protocol selector contract
- `USDC_ADDRESS`: USDC token contract
- `AAVE_ADAPTER_ADDRESS`: Aave protocol adapter
- `COMPOUND_ADAPTER_ADDRESS`: Compound protocol adapter
- `MORPHO_ADAPTER_ADDRESS`: Morpho protocol adapter

### Seeding (Optional)
- `RUN_SEEDS`: Enable database seeding (default: false)
- `SEED_MARKET_COUNT`: Number of markets to seed (default: 10)

## API Documentation

### Endpoints Overview

#### Core APIs
- `GET /api` - API information
- `GET /api/health` - Health check

#### Markets
- `GET /api/markets` - List markets with filtering and pagination
- `GET /api/markets/{id}` - Get market details
- `GET /api/markets/{id}/stats` - Get market statistics
- `POST /api/markets` - Create new market (admin)

#### Betting
- `GET /api/bets` - List bets with filtering
- `POST /api/bets` - Place a bet
- `GET /api/users/{address}/bets` - Get user bets

#### Users
- `GET /api/users/{address}` - Get user profile
- `GET /api/users/{address}/stats` - Get user statistics
- `PUT /api/users/{address}` - Update user profile

#### Protocols
- `GET /api/protocols` - List available yield protocols
- `GET /api/yields` - Get yield records

#### Charts & Analytics
- `GET /api/charts/{market_id}` - Get market chart data
- `GET /api/stats/platform` - Platform-wide statistics
- `GET /api/stats/leaderboard` - User leaderboard

#### Authentication
- `POST /api/auth/connect` - Connect wallet
- `POST /api/auth/refresh` - Refresh JWT token

### API Documentation UI

Access the interactive API documentation at:
- **Swagger UI**: `http://localhost:3002/api/swagger-ui/`
- **OpenAPI Spec**: `http://localhost:3002/api/openapi.json`

## Database Schema

The application uses PostgreSQL with the following main tables:

- `market_createds` - Market information
- `bet_placeds` - Betting records
- `market_resolveds` - Market resolution data
- `protocol_registereds` - Protocol registrations
- `yield_records` - Yield generation tracking
- `fee_records` - Fee tracking
- `users` - User profiles

## Development

### Running in Development

```bash
# Run with auto-reload
cargo watch -x run

# Run with database seeding
RUN_SEEDS=true cargo run

# Run tests
cargo test
```

### Code Structure

```
src/
├── admin/          # Admin panel routes
├── middleware/     # Authentication and JWT middleware
├── routes/         # API route handlers
│   ├── auth.rs     # Authentication endpoints
│   ├── bets.rs     # Betting endpoints
│   ├── markets.rs  # Market endpoints
│   ├── protocols.rs # Protocol endpoints
│   └── ...
├── services/       # Business logic services
├── models.rs       # Data models and schemas
├── config.rs       # Configuration management
├── db.rs          # Database connection
├── error.rs       # Error handling
└── main.rs        # Application entry point
```

### Database Migrations

Migrations are stored in the `migrations/` directory and are automatically applied on startup.

```bash
# Create new migration
sqlx migrate add <migration_name>

# Apply migrations manually
sqlx migrate run
```

## Deployment

### Production Build

```bash
# Build optimized release
cargo build --release

# Run production server
./target/release/whizy-base-server
```

### Docker Deployment

The application can be containerized for easy deployment:

```dockerfile
# Example Dockerfile structure
FROM rust:1.70 as builder
# Build steps...

FROM debian:bookworm-slim
# Runtime setup...
```

## Monitoring and Logging

The application uses structured logging with the `tracing` crate:

- Configure log levels via `RUST_LOG` environment variable
- Logs include request tracing and performance metrics
- Health check endpoint for monitoring system health

## Security

- JWT-based authentication for protected endpoints
- API key protection for administrative functions
- Input validation and sanitization
- Secure blockchain transaction handling
- CORS configuration for cross-origin requests

## Performance

- Built with Rust for high performance and memory safety
- Asynchronous request handling with Tokio
- Connection pooling for database operations
- Efficient blockchain synchronization
- Configurable CORS and compression

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Submit a pull request

## License

Licensed under the Apache License 2.0. See LICENSE file for details.

## Support

For questions and support:
- Create an issue in the repository
- Check the API documentation for endpoint details
- Review the configuration guide for setup help
