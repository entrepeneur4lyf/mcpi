#!/bin/bash

# Script to build and run the MCPI workspace

# Check if data directory exists
if [ ! -d "data" ]; then
    echo "Creating data directory..."
    mkdir -p data
fi

# Check if config.json exists
if [ ! -f "data/config.json" ]; then
    echo "Error: data/config.json is required but not found."
    echo "Please create the configuration file before running."
    exit 1
fi

# Build the workspace
echo "Building MCPI workspace..."
cargo build --workspace

# Check build status
if [ $? -ne 0 ]; then
    echo "Build failed!"
    exit 1
fi

# Run the server
echo "Starting MCPI server..."
echo "Open a new terminal window and run 'cargo run -p mcpi-client' to test the client"
cargo run -p mcpi-server