# MCPI Server Docker Deployment

## Overview

This document provides comprehensive guidance for deploying the MCPI server using Docker, ensuring consistent and reproducible deployments across different environments.

## Prerequisites

- Docker (20.10 or newer)
- Docker Compose (2.0 or newer)
- Git
- Basic understanding of containerization

## Directory Structure

```
mcpi/
└── docker/
    └── server/
        ├── Dockerfile
        └── docker-compose.yml
```

## Configuration Files

### Dockerfile

The Dockerfile uses a multi-stage build:
- First stage: Compiles the Rust application
- Second stage: Creates a minimal runtime image using distroless base

Key features:
- Uses official Rust image for building
- Leverages distroless image for minimal runtime
- Copies only necessary artifacts

### Docker Compose Configuration

The `docker-compose.yml` provides:
- Port mapping
- Volume mounts
- Environment variable configuration

## Deployment Steps

### 1. Clone the Repository

```bash
git clone https://github.com/McSpidey/mcpi.git
cd mcpi
```

### 2. Navigate to Docker Directory

```bash
cd docker/server
```

### 3. Build and Start the Server

```bash
docker-compose up --build -d
```

## Configuration Options

### Port Mapping

Modify `docker-compose.yml` to change port bindings:

```yaml
ports:
  - "3001:3001"  # Format: Host:Container
```

### Data Persistence

Use volume mounts to customize data:

```yaml
volumes:
  - ../../data:/app/data  # Maps local data directory
```

### Environment Variables

Adjust runtime behavior:

```yaml
environment:
  - RUST_LOG=info          # Logging level
  - RUST_BACKTRACE=1       # Enable detailed error traces
```

## Common Docker Commands

| Command | Description |
|---------|-------------|
| `docker-compose up --build -d` | Start server in detached mode |
| `docker-compose down` | Stop and remove containers |
| `docker-compose logs mcpi-server` | View server logs |
| `docker-compose ps` | List running containers |
| `docker-compose restart mcpi-server` | Restart the server |

## Accessing the Server

- **WebSocket**: `ws://localhost:3001/mcpi`
- **Discovery Endpoint**: `http://localhost:3001/mcpi/discover`
- **Admin Panel**: `http://localhost:3001/admin`

## Security Considerations

- Avoid exposing ports unnecessarily
- Use network restrictions
- Regularly update Docker and base images
- Consider using Docker secrets for sensitive configurations

## Troubleshooting

### Common Issues

1. **Port Already in Use**
   - Ensure port 3001 is not occupied by another service
   - Use `lsof -i :3001` to check port usage

2. **Connection Refused**
   - Verify Docker is running
   - Check firewall settings
   - Confirm container is up: `docker-compose ps`

3. **Performance Issues**
   - Allocate sufficient resources
   - Monitor container resource usage

### Logging and Debugging

- Enable verbose logging by adjusting `RUST_LOG`
- Use `docker-compose logs -f mcpi-server` for real-time logs

## Best Practices

- Use specific version tags for base images
- Minimize image size
- Avoid running containers as root
- Implement proper error handling
- Regular security scans

## Contributing

Found an issue with the Docker setup? Please:
1. Check existing issues
2. Create a detailed bug report
3. Submit a pull request with proposed fixes

## License

This Docker configuration is part of the MCPI project and follows the project's MIT License.