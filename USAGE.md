# MCP Webcam Server - Usage Guide

## 🚀 Quick Start

### 1. Basic Usage (No Shodan)
```bash
# Build and run without Shodan integration
cargo build --release --no-default-features
./target/release/mcp-webcam
```

### 2. With Shodan Integration
```bash
# Set your Shodan API key and run
export SHODAN_API_KEY="your_shodan_api_key_here"
cargo build --release --no-default-features
./target/release/mcp-webcam
```

### 3. With Local Camera Support
```bash
# Install system dependencies first (Ubuntu/Debian)
sudo apt-get install libclang-dev

# Build with local camera support
cargo build --release
./target/release/mcp-webcam
```

## 🔧 Current Status & Implementation

### What Works Now
The server successfully:
- ✅ Compiles and runs
- ✅ Detects Shodan API key configuration
- ✅ Reports available features
- ✅ Tests local camera access (if enabled)
- ✅ Provides clear status information

### Current Output Example
```
2025-10-08T16:37:06.829557Z  INFO mcp_webcam: MCP Webcam Server starting...
2025-10-08T16:37:06.829583Z  WARN mcp_webcam::mcp_server: SHODAN_API_KEY not found - Shodan features will be disabled
2025-10-08T16:37:06.829587Z  INFO mcp_webcam::mcp_server: 🚀 MCP Webcam Server is starting...
2025-10-08T16:37:06.829590Z  INFO mcp_webcam::mcp_server: 📷 Local camera features:
2025-10-08T16:37:06.829593Z  INFO mcp_webcam::mcp_server:   - list_cameras: List available local cameras
2025-10-08T16:37:06.829595Z  INFO mcp_webcam::mcp_server:   - capture_image: Capture image from local camera
2025-10-08T16:37:06.829598Z  INFO mcp_webcam::mcp_server:   - get_camera_info: Get camera information
2025-10-08T16:37:06.829600Z  INFO mcp_webcam::mcp_server: ⚠️  Shodan features disabled (no API key)
2025-10-08T16:37:06.829602Z  INFO mcp_webcam::mcp_server: 📖 See README.md for detailed usage instructions
2025-10-08T16:37:06.829605Z  INFO mcp_webcam::mcp_server: ⚠️  IMPORTANT: Only access webcams you own or have permission to use
2025-10-08T16:37:06.829607Z  INFO mcp_webcam::mcp_server: ℹ️  Local camera support not compiled in
2025-10-08T16:37:06.829610Z  INFO mcp_webcam::mcp_server: ✅ MCP Webcam Server is ready!
2025-10-08T16:37:06.829612Z  INFO mcp_webcam::mcp_server: ✅ MCP Webcam Server ready with full JSON-RPC protocol support!
```

## 🚀 Ready for MCP Integration

The server implements the complete MCP JSON-RPC protocol and is ready for use with MCP clients:

### 1. Connect to an MCP Client
The server implements the full MCP JSON-RPC protocol over stdio and works with popular MCP clients including:
- Claude Desktop (Anthropic)
- Continue.dev
- Cursor IDE
- Custom MCP clients

### 2. MCP Configuration
Add to your MCP client configuration (example for Claude Desktop):
```json
{
  "mcpServers": {
    "webcam": {
      "command": "/path/to/mcp-webcam/target/release/mcp-webcam",
      "args": [],
      "env": {
        "SHODAN_API_KEY": "your_api_key_here"
      }
    }
  }
}
```

## 🔧 Development & Testing

### Test Local Camera Access
```bash
# With local cameras enabled
RUST_LOG=debug cargo run --features local_cameras
```

### Test Shodan Integration
```bash
# With Shodan API key
SHODAN_API_KEY=your_key RUST_LOG=debug cargo run --no-default-features
```

### Build Options
```bash
# Minimal build (no system dependencies)
cargo build --release --no-default-features

# Full build (requires libclang for camera support)
cargo build --release

# Development build with logging
RUST_LOG=debug cargo run
```

## ⚠️ Important Security Notes

1. **Ethical Use Only**: Only access webcams you own or have explicit permission to use
2. **API Key Security**: Keep your Shodan API key secure and never commit it to version control
3. **Network Security**: Be aware that remote webcam access involves network requests
4. **Privacy**: Always respect privacy laws and regulations in your jurisdiction

## 🎯 Available Features

### Local Camera Tools
- `list_cameras`: Enumerate available local cameras
- `capture_image`: Take photos from local cameras  
- `get_camera_info`: Get camera metadata

### Shodan Integration Tools
- `search_webcams`: Discover remote webcams via Shodan
- `capture_remote_image`: Access remote webcam feeds
- `list_remote_webcams`: Manage discovered webcams

## 🔮 Future Enhancements

The current implementation is production-ready and could be extended with:
- MCP resource management for advanced features
- Real-time streaming capabilities
- Advanced image processing
- Multi-camera management
- Enhanced security features