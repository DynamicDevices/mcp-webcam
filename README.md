# MCP Webcam Server

A Model Context Protocol (MCP) server implementation in Rust that provides both local and remote webcam access to AI assistants.

## ⚠️ IMPORTANT SECURITY AND ETHICAL NOTICE

**This software includes functionality to discover and access internet-connected webcams via Shodan. Please read and understand the following:**

### Legal and Ethical Considerations
- **Only access webcams you own or have explicit permission to access**
- **Unauthorized access to webcams is illegal in most jurisdictions**
- **Always respect privacy and obtain proper consent**
- **Use this tool responsibly and ethically**
- **Consider the privacy implications of accessing remote cameras**

### Security Warnings
- Remote webcam access may expose you to security risks
- Always verify the legitimacy of webcam sources
- Be aware that some discovered cameras may be honeypots or compromised systems
- Use appropriate network security measures when accessing remote cameras

## Features

- **Local webcam access** using `nokhwa` crate (cross-platform)
- **Remote webcam discovery** via Shodan API integration
- **Remote webcam access** with HTTP/MJPEG support
- **MCP-compliant server** with stdio transport
- **Multiple camera support** - list and select from available cameras
- **Image capture** with automatic JPEG encoding and base64 output
- **Comprehensive error handling** and logging
- **Real-time camera information** and status

## Available Tools

### Local Camera Tools

### `list_cameras`
Lists all available local camera devices on the system.

**Parameters:** None

**Returns:**
```json
{
  "cameras": [
    {
      "index": 0,
      "name": "Integrated Camera",
      "description": "USB Video Device",
      "available": true
    }
  ]
}
```

### `capture_image`
Captures an image from the specified local camera (or default camera if not specified).

**Parameters:**
- `camera_index` (optional): Camera index to use (defaults to 0)

**Returns:**
```json
{
  "content": [
    {
      "type": "image",
      "data": "base64-encoded-jpeg-data",
      "mimeType": "image/jpeg"
    },
    {
      "type": "text", 
      "text": "Captured 1920x1080 image from camera 0 at 2024-01-01T12:00:00Z"
    }
  ],
  "metadata": {
    "width": 1920,
    "height": 1080,
    "camera_index": 0,
    "timestamp": "2024-01-01T12:00:00Z",
    "mime_type": "image/jpeg"
  }
}
```

### `get_camera_info`
Gets detailed information about all available local cameras and current status.

**Parameters:** None

### Remote Webcam Tools (Shodan Integration)

⚠️ **These tools require a Shodan API key and should be used responsibly**

### `search_webcams`
Search for internet-connected webcams using Shodan.

**Parameters:**
- `limit` (optional): Maximum number of results to return (default: 20)

**Returns:**
```json
{
  "content": [{
    "type": "text",
    "text": "Found 15 remote webcam(s) via Shodan search"
  }],
  "webcams": [
    {
      "ip": "192.168.1.100",
      "port": 8080,
      "url": "http://192.168.1.100:8080/mjpeg",
      "hostname": "example.com",
      "location": {
        "country_name": "United States",
        "city": "New York"
      },
      "org": "Example ISP",
      "access_type": "MJPEG"
    }
  ],
  "total": 15
}
```

### `capture_remote_image`
Capture an image from a remote webcam.

**Parameters:**
- `url` (required): Full URL to the webcam stream

**Returns:**
```json
{
  "content": [
    {
      "type": "image",
      "data": "base64-encoded-image-data",
      "mimeType": "image/jpeg"
    },
    {
      "type": "text",
      "text": "Captured image from remote webcam: http://example.com/mjpeg"
    }
  ]
}
```

### `list_remote_webcams`
Lists previously discovered remote webcams (placeholder for caching functionality).

## Installation

### Prerequisites

- Rust 1.70+ (install via [rustup](https://rustup.rs/))
- Local webcam/camera device (for local functionality)
- **Shodan API key** (for remote webcam discovery - optional)
- Platform-specific requirements:
  - **Linux**: Video4Linux2 support (`v4l2` drivers)
  - **Windows**: DirectShow/Media Foundation
  - **macOS**: AVFoundation

### Getting a Shodan API Key

1. Sign up at [shodan.io](https://www.shodan.io/)
2. Go to your account page to find your API key
3. Set the environment variable: `export SHODAN_API_KEY=your_api_key_here`

**Note:** Shodan functionality is optional. The server will work without it for local camera access.

### Build from Source

```bash
git clone <repository-url>
cd mcp-webcam
cargo build --release
```

## Usage

### As MCP Server (Recommended)

The server communicates via stdio, making it easy to integrate with MCP clients:

```bash
# Run the server
cargo run --release

# Or use the binary directly
./target/release/mcp-webcam
```

### Integration with AI Assistants

Add to your MCP client configuration (e.g., Claude Desktop):

```json
{
  "mcpServers": {
    "webcam": {
      "command": "/path/to/mcp-webcam",
      "args": []
    }
  }
}
```

### Environment Variables

- `RUST_LOG`: Set logging level (e.g., `RUST_LOG=mcp_webcam=debug`)
- `SHODAN_API_KEY`: Your Shodan API key for remote webcam discovery (optional)

### Example Usage

```bash
# Run with local cameras only
cargo run --release

# Run with Shodan integration
SHODAN_API_KEY=your_key_here cargo run --release

# With debug logging
RUST_LOG=mcp_webcam=debug SHODAN_API_KEY=your_key_here cargo run --release
```

## Development

### Running in Development

```bash
# With debug logging
RUST_LOG=mcp_webcam=debug cargo run

# Run tests
cargo test

# Check code
cargo clippy
cargo fmt
```

### Project Structure

```
src/
├── main.rs           # Application entry point
├── lib.rs            # Library exports
├── webcam.rs         # Local webcam capture logic
├── shodan.rs         # Shodan API integration and remote webcam access
└── mcp_server.rs     # MCP server implementation
```

## Troubleshooting

### Local Camera Issues

1. **Permission denied**: Ensure your user has access to video devices
   ```bash
   # Linux: Add user to video group
   sudo usermod -a -G video $USER
   ```

2. **No cameras found**: 
   - Check if camera is connected and working
   - Verify drivers are installed
   - Test with other applications (e.g., `cheese` on Linux)

3. **Camera busy**: Close other applications using the camera

### Shodan Integration Issues

1. **"No Shodan API key" error**: 
   - Set the `SHODAN_API_KEY` environment variable
   - Verify your API key is correct

2. **Rate limit exceeded**:
   - Shodan has API rate limits
   - Wait before making more requests
   - Consider upgrading your Shodan plan

3. **Unauthorized error**:
   - Check your API key is valid
   - Ensure your Shodan account is active

4. **Remote webcam access fails**:
   - Many discovered webcams may not be accessible
   - Some may require authentication
   - Network firewalls may block access
   - The webcam may be offline

### Ethical and Legal Considerations

**IMPORTANT**: Only access webcams you own or have explicit permission to access. Unauthorized access is illegal and unethical.

### Logging

Enable debug logging to troubleshoot issues:

```bash
RUST_LOG=mcp_webcam=debug,nokhwa=debug,reqwest=debug cargo run
```

## Dependencies

- **mcpr**: MCP protocol implementation
- **nokhwa**: Cross-platform webcam access
- **reqwest**: HTTP client for Shodan API and remote webcam access
- **tokio**: Async runtime
- **image**: Image processing and encoding
- **base64**: Image data encoding
- **tracing**: Structured logging
- **chrono**: Date/time handling
- **serde**: Serialization framework

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

[Add your license here]

## Changelog

### v0.1.0
- Initial release
- Local webcam capture functionality
- MCP server implementation
- Cross-platform camera support
- **Shodan integration for remote webcam discovery**
- **Remote webcam image capture**
- **Comprehensive safety and ethical usage warnings**
