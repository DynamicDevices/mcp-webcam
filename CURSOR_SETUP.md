# 🎯 Testing MCP Webcam Server with Cursor IDE

## 🚀 Quick Setup Guide

### Step 1: Verify Server Binary
First, ensure your MCP server is built and ready:

```bash
cd /home/ajlennon/data_drive/dd/mcp-webcam
ls -la target/release/mcp-webcam
```

The binary should be executable. If not built yet:
```bash
cargo build --release --no-default-features
```

### Step 2: Configure Cursor MCP Integration

1. **Open Cursor Settings**:
   - Launch Cursor IDE
   - Navigate to `Settings` > `Features` > `MCP`

2. **Add MCP Server**:
   - Click `+ Add New MCP Server`
   - Fill in the configuration:

```json
{
  "mcpServers": {
    "Webcam Server": {
      "command": "/home/ajlennon/data_drive/dd/mcp-webcam/target/release/mcp-webcam",
      "args": [],
      "env": {
        "SHODAN_API_KEY": "your_shodan_api_key_here",
        "RUST_LOG": "info"
      }
    }
  }
}
```

**Configuration Details:**
- **Name**: `Webcam Server` (or any descriptive name)
- **Type**: `stdio` (transport method)
- **Command**: Full absolute path to your binary
- **Arguments**: Empty array `[]`
- **Environment Variables**:
  - `SHODAN_API_KEY`: Your Shodan API key (optional, for remote webcam features)
  - `RUST_LOG`: Set to `info` or `debug` for logging

### Step 3: Save and Verify

1. **Save Configuration**: Click save in the MCP settings
2. **Refresh Server**: Click the refresh icon next to your server
3. **Check Status**: Ensure the server shows as "running" without errors

### Step 4: Test Integration

1. **Open Chat Panel**: 
   - Toggle chat panel (top-right corner)
   - Or use shortcut: `⌘+i` (macOS) / `Ctrl+i` (Windows/Linux)

2. **Switch to Agent Mode**: Ensure you're in `Agent` mode

3. **Test Commands**: Try these prompts:

## 🧪 Test Commands

### Test 1: List Local Cameras
```
Can you list all available cameras on this system?
```
*Expected: The agent should call the `list_cameras` tool*

### Test 2: Get Camera Information
```
What camera information is available on this system?
```
*Expected: The agent should call the `get_camera_info` tool*

### Test 3: Capture Local Image (if cameras available)
```
Can you take a photo using the default camera?
```
*Expected: The agent should call the `capture_image` tool*

### Test 4: Search Remote Webcams (requires Shodan API key)
```
Search for remote webcams using Shodan, limit to 5 results
```
*Expected: The agent should call the `search_webcams` tool*

### Test 5: Test Remote Image Capture (requires valid webcam URL)
```
Capture an image from this webcam URL: http://example.com/mjpeg
```
*Expected: The agent should call the `capture_remote_image` tool*

## 🔍 Troubleshooting

### Common Issues

**1. Server Not Starting**
- Check the binary path is correct and absolute
- Verify the binary is executable: `chmod +x target/release/mcp-webcam`
- Check Cursor's MCP logs for error messages

**2. Permission Denied (Local Cameras)**
```bash
# Add user to video group (Linux)
sudo usermod -a -G video $USER
# Then log out and back in
```

**3. Shodan Features Not Available**
- Ensure `SHODAN_API_KEY` is set in the environment variables
- Verify your Shodan API key is valid
- Check the server logs for Shodan-related messages

**4. Tools Not Appearing**
- Refresh the MCP server in Cursor settings
- Check that the server binary runs without errors:
```bash
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' | ./target/release/mcp-webcam
```

### Debug Mode

For detailed troubleshooting, set debug logging:
```json
{
  "env": {
    "RUST_LOG": "debug",
    "SHODAN_API_KEY": "your_key_here"
  }
}
```

### Manual Server Test

Test the server manually to ensure it's working:
```bash
# Test server startup
timeout 3s ./target/release/mcp-webcam

# Should show server starting and tool registration messages
```

## ✅ Expected Results

When working correctly, you should see:

1. **Server Status**: "Running" in Cursor MCP settings
2. **Available Tools**: 5 tools available (3 local camera + 2 Shodan tools if API key provided)
3. **Agent Integration**: Cursor's agent can call webcam tools when prompted
4. **Image Capture**: Base64-encoded images returned from successful captures
5. **Error Handling**: Graceful error messages for failed operations

## 🎉 Success Indicators

- ✅ Server appears as "Running" in Cursor MCP settings
- ✅ Agent can list available cameras
- ✅ Agent can capture images (if cameras available)
- ✅ Agent can search for remote webcams (if Shodan key provided)
- ✅ Proper error messages for unavailable features
- ✅ Base64 image data returned for successful captures

## 📝 Notes

- **Local cameras**: May not be available on all systems (especially servers/VMs)
- **Remote webcams**: Requires Shodan API key and should only be used ethically
- **Image format**: All images are returned as base64-encoded JPEG data
- **Async operations**: Shodan searches may take a few seconds to complete
