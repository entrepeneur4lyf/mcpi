# MCPI Architecture Overview

## System Design Principles

### Core Architecture
- **Protocol Layer**: Standardized communication interface
- **Plugin System**: Modular, extensible capability framework
- **Discovery Mechanism**: Flexible service detection

### Component Interactions

#### mcpi-common
- Defines core traits and interfaces
- Provides shared data structures
- Implements generic plugin mechanisms

#### mcpi-server
- Hosts WebSocket server
- Manages plugin registry
- Handles protocol-level interactions

#### mcpi-client
- Reference implementation of client interactions
- Demonstrates protocol usage
- Provides discovery and connection utilities

## Plugin Architecture

### Plugin Types
- **Core Plugins**: Built-in essential services
- **Extension Plugins**: Optional, dynamically loadable capabilities

### Plugin Interface Requirements
- Implement `McpPlugin` trait
- Define supported operations
- Provide input schema
- Implement execution logic

## Discovery Mechanisms

### DNS-Based Discovery
- Uses TXT records for service description
- Supports version negotiation
- Provides lightweight service metadata

### WebSocket Protocol
- Standardized JSON-RPC communication
- Supports dynamic capability enumeration
- Provides stateful interaction model

## Design Goals
- Minimimal overhead
- Maximum flexibility
- Clear separation of concerns
- Easy extensibility