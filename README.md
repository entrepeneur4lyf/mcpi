# MCPI: Model Context Protocol Integration

MCPI (Model Context Protocol Integration) is an implementation of the Model Context Protocol (MCP) for AI-web connectivity. It enables AI agents to discover, verify, and transact with web services through a standardized protocol.

**Official Repository:** [https://github.com/McSpidey/mcpi](https://github.com/McSpidey/mcpi)

## Overview

MCPI extends the Model Context Protocol to create a bridge between AI agents and web services. This implementation provides:

- WebSocket-based MCP protocol communication
- RESTful discovery endpoint
- DNS-based service discovery
- Data-driven capability definition
- Generic operation handlers (SEARCH, GET, LIST)
- Referral relationships between services

## Project Structure

```
mcpi/
├── Cargo.toml                # Workspace configuration
├── data/                     # Data directory for server
│   ├── config.json           # Main configuration file
│   ├── products.json         # Product catalog
│   ├── customers.json        # Customer information
│   ├── orders.json           # Order history
│   └── reviews.json          # Product reviews
├── mcpi-common/              # Shared types and utilities
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs            # Common types and utilities
├── mcpi-server/              # MCPI server implementation
│   ├── Cargo.toml
│   └── src/
│       └── main.rs           # Server implementation
└── mcpi-client/              # MCPI client example
    ├── Cargo.toml
    └── src/
        ├── main.rs           # Client implementation
        └── discovery.rs      # DNS discovery utilities
```

## Prerequisites

- Rust and Cargo (2021 edition or newer)
- Internet connection for dependencies
- Dig command-line tool (for DNS discovery)

## Getting Started

### Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/McSpidey/mcpi.git
   cd mcpi
   ```

2. Create the `data` directory in the workspace root:
   ```bash
   mkdir -p data
   ```

3. Set up the required data files as described in the "Data Files" section below.

### Building

Build the entire workspace:

```bash
cargo build --workspace
```

Or build individual components:

```bash
cargo build -p mcpi-server
cargo build -p mcpi-client
```

### Running the Server

```bash
cargo run -p mcpi-server
```

The server will start at `http://localhost:3000` with the following endpoints:
- WebSocket endpoint: `ws://localhost:3000/mcpi`
- REST discovery endpoint: `http://localhost:3000/mcpi/discover`

### Running the Client

The client has several options for connecting to MCPI servers:

#### Connect to local server (default):

```bash
cargo run -p mcpi-client
```

#### Discover server via DNS:

```bash
cargo run -p mcpi-client -- --domain example.com
```

#### Connect to specific URL:

```bash
cargo run -p mcpi-client -- --url ws://example.com/mcpi
```

## Data Files

The server requires a specific set of JSON files in the `data` directory to operate. These files define the server's configuration, capabilities, and data.

### 1. `config.json` (Required)

Main configuration file defining provider info, capabilities, and referrals:

```json
{
  "provider": {
    "name": "Example Store",
    "domain": "example.com",
    "description": "Online retailer of eco-friendly products",
    "branding": {
      "colors": {
        "primary": "#3498db",
        "secondary": "#2ecc71"
      },
      "logo": {
        "vector": "https://example.com/logo.svg"
      },
      "typography": {
        "primary": "Helvetica Neue"
      },
      "tone": "professional"
    }
  },
  "referrals": [
    {
      "name": "Eco Shipping",
      "domain": "ecoshipping.com",
      "relationship": "trusted"
    },
    {
      "name": "Green Packaging",
      "domain": "greenpack.co",
      "relationship": "partner"
    }
  ],
  "capabilities": {
    "product_search": {
      "name": "product_search",
      "description": "Search for products in catalog",
      "category": "inventory",
      "operations": ["SEARCH", "GET", "LIST"],
      "data_file": "products.json"
    },
    "customer_lookup": {
      "name": "customer_lookup",
      "description": "Look up customer information",
      "category": "customers",
      "operations": ["GET", "LIST"],
      "data_file": "customers.json"
    },
    "order_history": {
      "name": "order_history",
      "description": "Retrieve customer order history",
      "category": "orders",
      "operations": ["GET", "LIST", "SEARCH"],
      "data_file": "orders.json"
    },
    "product_reviews": {
      "name": "product_reviews",
      "description": "Get and submit product reviews",
      "category": "reviews",
      "operations": ["GET", "LIST", "SEARCH"],
      "data_file": "reviews.json"
    }
  }
}
```

### 2. Data Files (Required for each capability)

Each capability references a data file that contains its data. The server validates that all referenced files exist before starting.

Example data file formats:

#### `products.json`
```json
[
  {
    "id": "eco-1001",
    "name": "Bamboo Water Bottle",
    "price": 24.99,
    "description": "Eco-friendly bamboo water bottle",
    "inStock": true,
    "rating": 4.5,
    "categories": ["drinkware", "sustainable"],
    "materials": ["bamboo", "stainless steel"]
  }
]
```

#### `customers.json`
```json
[
  {
    "id": "cust-1001",
    "name": "Jane Smith",
    "email": "jane.smith@example.com",
    "tier": "premium",
    "since": "2023-05-15",
    "preferences": {
      "notifications": true,
      "theme": "dark"
    }
  }
]
```

## DNS-Based Discovery

MCPI supports DNS-based discovery that allows clients to find MCPI servers using DNS TXT records:

### Setting Up DNS TXT Records

1. Create a TXT record for your domain:
   - **NAME**: `_mcp` 
   - **TYPE**: `TXT`
   - **CONTENT**: `v=mcp1 url=https://api.example.com/mcpi/discover`

### Discovery Process

1. Client queries DNS for `_mcp.example.com` TXT record
2. Client extracts the discovery URL (`url=...`)
3. Client makes HTTP request to this discovery URL
4. Server responds with full MCPI capabilities
5. Client connects to WebSocket endpoint for MCP protocol

### Testing DNS Setup

Test your DNS TXT record using the `dig` command:

```bash
dig +short TXT _mcp.example.com
```

## MCP Protocol Implementation

This implementation follows the Model Context Protocol (MCP) specification:

### Supported MCP Methods

- `initialize`: Initialize the connection
- `resources/list`: List available resources
- `resources/read`: Read a specific resource
- `tools/list`: List available tools 
- `tools/call`: Execute a capability
- `ping`: Check connection health

### Example Request/Response

Initialize request:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "clientInfo": {
      "name": "MCPI Test Client",
      "version": "0.1.0"
    },
    "protocolVersion": "0.1.0",
    "capabilities": {
      "sampling": {}
    }
  }
}
```

Tool call request:
```json
{
  "jsonrpc": "2.0",
  "id": 5,
  "method": "tools/call",
  "params": {
    "name": "product_search",
    "arguments": {
      "operation": "SEARCH",
      "query": "bamboo"
    }
  }
}
```

## Adding New Capabilities

To add a new capability to the server:

1. Create a data file for the capability (e.g., `new_capability.json`)
2. Add the capability definition to `config.json`:
   ```json
   "new_capability": {
     "name": "new_capability",
     "description": "Description of new capability",
     "category": "category_name",
     "operations": ["SEARCH", "GET", "LIST"],
     "data_file": "new_capability.json"
   }
   ```
3. Restart the server

## Architecture

### Server Design

The server implements a generic capability execution engine that:
1. Loads capability definitions from `config.json`
2. Validates that all data files exist
3. Exposes capabilities as both resources and tools
4. Handles standard operations (SEARCH, GET, LIST) generically
5. Allows for complete separation of code and data

### Client Design

The client implements:
1. DNS-based discovery
2. HTTP-based capability discovery
3. WebSocket-based MCP protocol communication
4. Command-line interface with multiple connection options

## Extending the Implementation

This implementation can be extended in several ways:

1. **Authentication**: Add JWT or OAuth authentication
2. **Custom Operations**: Add non-standard operations beyond SEARCH, GET, LIST
3. **Database Integration**: Replace file-based storage with database access
4. **Caching**: Add caching for improved performance
5. **Economic Framework**: Implement USDC-based reverse fees described in MCPI spec

## Troubleshooting

### Server Issues

- **Missing Data Files**: Ensure all data files referenced in `config.json` exist in the `data` directory
- **JSON Formatting**: Validate JSON files using a tool like `jq`
- **Permission Issues**: Ensure the server has read access to the data files

### Client Issues

- **DNS Discovery**: Verify TXT records with `dig +short TXT _mcp.example.com`
- **Connection Errors**: Check network connectivity and server status
- **WebSocket Issues**: Verify that the server's WebSocket endpoint is accessible

## Contributing

We welcome contributions to the MCPI project! To contribute:

1. Fork the repository on GitHub
2. Create a feature branch
3. Implement your changes
4. Submit a pull request

Please ensure your code follows the project's style and includes appropriate tests.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- Based on the Model Context Protocol specification
- Inspired by the need for standardized AI-web connectivity