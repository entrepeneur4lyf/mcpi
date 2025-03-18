# MCPI Workspace

This is a Rust workspace for the Model Context Protocol Integration (MCPI) server and client, implementing the MCP (Model Context Protocol) specification for communication between AI models and web services.

## Workspace Structure

```
mcpi-workspace/
├── Cargo.toml                      # Workspace configuration
├── data/                           # Data directory for server
│   ├── config.json                 # Server configuration
│   ├── products.json               # Product data
│   ├── customers.json              # Customer data
│   └── ...                         # Other data files
├── mcpi-common/                    # Shared code
│   ├── Cargo.toml                  # Common dependencies
│   └── src/
│       └── lib.rs                  # Common types and utilities
├── mcpi-server/                    # MCPI server implementation
│   ├── Cargo.toml                  # Server dependencies
│   └── src/
│       └── main.rs                 # Server implementation
└── mcpi-client/                    # MCPI client example
    ├── Cargo.toml                  # Client dependencies
    └── src/
        └── main.rs                 # Client implementation
```

## Overview

This workspace implements:

1. **MCPI Server** - A server implementing both:
   - REST discovery endpoint at `/mcpi/discover`
   - MCP-compliant WebSocket endpoint at `/mcpi`

2. **MCPI Client** - An example client that:
   - Uses the REST discovery endpoint
   - Connects via WebSocket for MCP-based communication
   - Demonstrates MCP protocol flow

3. **Common Code** - Shared types and utilities:
   - MCP protocol message types
   - MCPI configuration structures
   - Capability definitions

## Getting Started

### Prerequisites

- Rust and Cargo
- Internet connection for dependencies

### Setup

1. Clone the repository

2. Create the `data` directory in the workspace root:
   ```
   mkdir -p data
   ```

3. Create the required data files (see "Data Files" section)

### Running the Server

```bash
cargo run -p mcpi-server
```

The server will validate that all required files exist before starting.

### Running the Client Example

```bash
cargo run -p mcpi-client
```

## Data Files

### Required Files

1. `data/config.json` - Main configuration defining provider info and capabilities
2. Data files for each capability as specified in `config.json`

### config.json Structure

```json
{
  "provider": {
    "name": "Your Service Name",
    "domain": "yourdomain.com",
    "description": "Service description",
    "branding": { ... }
  },
  "referrals": [ ... ],
  "capabilities": {
    "capability_name": {
      "name": "capability_name",
      "description": "Capability description",
      "category": "category_name",
      "operations": ["OPERATION1", "OPERATION2"],
      "data_file": "data_filename.json"
    }
  }
}
```

## MCP Protocol Implementation

This implementation follows the MCP specification, supporting:

### Supported MCP Methods

- `initialize`: Initialize the connection with capabilities negotiation
- `ping`: Check connection health
- `resources/list`: List available resources
- `resources/read`: Read a specific resource
- `tools/list`: List available tools
- `tools/call`: Call a tool with arguments

### JSON-RPC Format

All MCP communication follows the JSON-RPC 2.0 format:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "resources/list",
  "params": { ... }
}
```

## DNS Configuration

For MCPI discovery, set up a DNS TXT record:

```
_mcp.example.com. IN TXT "v=mcp1 endpoints=wss://mcp1.example.com/mcpi,wss://mcp2.example.com/mcpi capabilities=product_search,customer_lookup,order_history,product_reviews"
```

## Adding New Capabilities

To add a new capability:

1. Create a data file for the capability (e.g., `data/new_capability.json`)
2. Add the capability definition to `config.json`

## Building for Production

```bash
cargo build --release --workspace
```

This will build optimized binaries for both server and client in the `target/release` directory.

## License

MIT License# MCPI Workspace

This is a Rust workspace for the Model Context Protocol Integration (MCPI) server and client, implementing the MCP (Model Context Protocol) specification for communication between AI models and web services.

## Workspace Structure

```
mcpi-workspace/
├── Cargo.toml                      # Workspace configuration
├── data/                           # Data directory for server
│   ├── config.json                 # Server configuration
│   ├── products.json               # Product data
│   ├── customers.json              # Customer data
│   └── ...                         # Other data files
├── mcpi-common/                    # Shared code
│   ├── Cargo.toml                  # Common dependencies
│   └── src/
│       └── lib.rs                  # Common types and utilities
├── mcpi-server/                    # MCPI server implementation
│   ├── Cargo.toml                  # Server dependencies
│   └── src/
│       └── main.rs                 # Server implementation
└── mcpi-client/                    # MCPI client example
    ├── Cargo.toml                  # Client dependencies
    └── src/
        └── main.rs                 # Client implementation
```

## Overview

This workspace implements:

1. **MCPI Server** - A server implementing both:
   - REST discovery endpoint at `/mcpi/discover`
   - MCP-compliant WebSocket endpoint at `/mcpi`

2. **MCPI Client** - An example client that:
   - Uses the REST discovery endpoint
   - Connects via WebSocket for MCP-based communication
   - Demonstrates MCP protocol flow

3. **Common Code** - Shared types and utilities:
   - MCP protocol message types
   - MCPI configuration structures
   - Capability definitions

## Getting Started

### Prerequisites

- Rust and Cargo
- Internet connection for dependencies

### Setup

1. Clone the repository

2. Create the `data` directory in the workspace root:
   ```
   mkdir -p data
   ```

3. Create the required data files (see "Data Files" section)

### Running the Server

```bash
cargo run -p mcpi-server
```

The server will validate that all required files exist before starting.

### Running the Client Example

```bash
cargo run -p mcpi-client
```

## Data Files

### Required Files

1. `data/config.json` - Main configuration defining provider info and capabilities
2. Data files for each capability as specified in `config.json`

### config.json Structure

```json
{
  "provider": {
    "name": "Your Service Name",
    "domain": "yourdomain.com",
    "description": "Service description",
    "branding": { ... }
  },
  "referrals": [ ... ],
  "capabilities": {
    "capability_name": {
      "name": "capability_name",
      "description": "Capability description",
      "category": "category_name",
      "operations": ["OPERATION1", "OPERATION2"],
      "data_file": "data_filename.json"
    }
  }
}
```

## MCP Protocol Implementation

This implementation follows the MCP specification, supporting:

### Supported MCP Methods

- `initialize`: Initialize the connection with capabilities negotiation
- `ping`: Check connection health
- `resources/list`: List available resources
- `resources/read`: Read a specific resource
- `tools/list`: List available tools
- `tools/call`: Call a tool with arguments

### JSON-RPC Format

All MCP communication follows the JSON-RPC 2.0 format:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "resources/list",
  "params": { ... }
}
```

## DNS Configuration

For MCPI discovery, set up a DNS TXT record:

```
_mcp.example.com. IN TXT "v=mcp1 endpoints=wss://mcp1.example.com/mcpi,wss://mcp2.example.com/mcpi capabilities=product_search,customer_lookup,order_history,product_reviews"
```

## Adding New Capabilities

To add a new capability:

1. Create a data file for the capability (e.g., `data/new_capability.json`)
2. Add the capability definition to `config.json`

## Building for Production

```bash
cargo build --release --workspace
```

This will build optimized binaries for both server and client in the `target/release` directory.

## License

MIT License