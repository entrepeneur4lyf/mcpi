# MCPI: Model Context Protocol Integration

MCPI (Model Context Protocol Integration) is an implementation of the Model Context Protocol (MCP) for AI-web connectivity. It enables AI agents to discover, verify, and transact with web services through a standardized protocol, now featuring a flexible plugin architecture.

**Official Repository:** [https://github.com/McSpidey/mcpi](https://github.com/McSpidey/mcpi)

## Overview

MCPI extends the Model Context Protocol to create a bridge between AI agents and web services. This implementation provides:

- WebSocket-based MCP protocol communication
- RESTful discovery endpoint
- DNS-based service discovery
- Plugin architecture for modular capabilities
- Generic operation handlers (SEARCH, GET, LIST)
- Referral relationships between services

## Plugin Architecture

The MCPI system uses a plugin architecture that allows for modular and extensible capabilities:

- **Plugins**: Each capability is implemented as a self-contained plugin
- **Plugin Registry**: Central management of all available plugins
- **Dynamic Operation**: Plugins can be loaded and configured at runtime
- **Extensibility**: New capabilities can be added without modifying core code

### Built-in Plugins

MCPI comes with several built-in plugins:

- **Website Plugin**: Provides e-commerce capabilities (products, customers, orders, reviews)
- **Weather Plugin**: Demonstrates dynamic data generation with simulated weather forecasts

### Custom Plugins

You can easily extend MCPI by creating your own plugins. Each plugin implements the McpPlugin trait, which defines methods for:

- Getting metadata (name, description, category)
- Listing supported operations
- Defining input schemas
- Executing operations
- Providing resources

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

The server will start at `http://localhost:3001` with the following endpoints:
- WebSocket endpoint: `ws://localhost:3001/mcpi`
- REST discovery endpoint: `http://localhost:3001/mcpi/discover`

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

#### Test a specific plugin:

```bash
cargo run -p mcpi-client -- --plugin weather_forecast
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
    }
  }
}
```

### 2. Data Files (Required for each capability)

Each capability references a data file that contains its data. These files should be placed in the `data/mock` directory.

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

## Creating New Plugins

To create a new plugin:

1. Create a new file in `mcpi-server/src/plugins/` (e.g., `my_plugin.rs`)
2. Implement the McpPlugin trait
3. Register your plugin in the server's main function

Plugins can be of different types:

1. **Data-driven plugins**: Extend the JsonDataPlugin base class
2. **Dynamic plugins**: Implement the McpPlugin trait directly
3. **API wrapper plugins**: Connect to external services

## Standard Plugin Operations

While plugins can define custom operations, these standard operations are recommended:

- **SEARCH**: Find items matching criteria
- **GET**: Retrieve a specific item by ID
- **LIST**: List all available items
- **CREATE**: Create a new item (for writable plugins)
- **UPDATE**: Update an existing item (for writable plugins)
- **DELETE**: Remove an item (for writable plugins)

## Architecture

### Server Design

The server implements a plugin-based capability execution engine that:
1. Loads the plugin registry
2. Registers built-in and configured plugins
3. Routes MCP protocol methods to appropriate plugins
4. Handles WebSocket connections and JSON-RPC requests
5. Provides plugin discovery and introspection

### Client Design

The client implements:
1. DNS-based discovery
2. HTTP-based capability discovery
3. WebSocket-based MCP protocol communication
4. Plugin-specific testing through command-line interface

## Extending the Implementation

This implementation can be extended in several ways:

1. **New Plugins**: Add new capabilities by creating new plugins
2. **Authentication**: Add JWT or OAuth authentication
3. **Database Integration**: Replace file-based storage with database access
4. **Caching**: Add caching for improved performance
5. **Economic Framework**: Implement USDC-based reverse fees described in MCPI spec

## Troubleshooting

### Server Issues

- **Missing Data Files**: Ensure all data files referenced in `config.json` exist in the `data/mock` directory
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