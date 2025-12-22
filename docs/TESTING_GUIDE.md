# End-to-End Testing Guide

This guide walks through testing the complete Pied Piper workflow from deployment to accessing via the gateway.

## Prerequisites

```bash
# Build the project
cd /Users/rootkill/pied-piper
cargo build --release

# Build the hello-api example
cd examples/hello-api
cargo build --target wasm32-wasip2 --release
cd ../..
```

## Step 1: Deploy the WASM Module

```bash
./target/release/pied-piper deploy ./examples/hello-api/target/wasm32-wasip2/release/hello_api.wasm
```

**Expected Output:**
```
✅ Module deployed successfully!
📦 Module Name: hello_api
🔗 CID: boc5trztwckf5jmqbl3o2p3ynmdfnz3vyvvhlxfctybxpcnsjuc2q
🆔 Provider Peer ID: 12D3KooW...
```

**Note:** You may see a warning about "no peers available" - this is normal when running as a single node. The module is still stored locally and available for serving.

**Save the CID** - you'll need it for the next steps!

## Step 2: Start the HTTP Gateway

```bash
# Start on port 3000 (or any available port)
./target/release/pied-piper gateway --listen 127.0.0.1:3000
```

**Expected Output:**
```
INFO pied_piper::gateway::server: Starting HTTP gateway on 127.0.0.1:3000
INFO pied_piper::gateway::server: Gateway is ready
```

Keep this terminal open - the gateway needs to stay running.

## Step 3: Test the API Endpoints

Open a new terminal and test the endpoints:

### Test 1: Hello Endpoint

```bash
curl "http://localhost:3000/cid/<YOUR_CID>/api/hello?name=Alice"
```

**Expected Response:**
```json
{
  "message": "Hello, Alice! 👋",
  "path": "/api/hello",
  "method": "GET"
}
```

### Test 2: Echo Endpoint

```bash
curl -X POST http://localhost:3000/cid/<YOUR_CID>/api/echo \
  -H "Content-Type: application/json" \
  -d '{"test": "data", "message": "Hello from client"}'
```

**Expected Response:**
```json
{
  "echo": "{\"test\":\"data\",\"message\":\"Hello from client\"}",
  "method": "POST",
  "path": "/api/echo",
  "content_type": "application/json",
  "body_length": 45
}
```

### Test 3: API Info

```bash
curl http://localhost:3000/cid/<YOUR_CID>/api/info
```

**Expected Response:**
```json
{
  "name": "hello-api",
  "version": "1.0.0",
  "description": "Example WASM API handler for Pied Piper",
  "endpoints": [
    {
      "method": "GET",
      "path": "/api/hello",
      "query_params": ["name"],
      "description": "Returns a greeting message"
    },
    ...
  ],
  "powered_by": "Pied Piper - Decentralized Internet Platform"
}
```

### Test 4: Health Check

```bash
curl http://localhost:3000/cid/<YOUR_CID>/api/health
```

**Expected Response:**
```json
{
  "status": "healthy"
}
```

## Step 4: Test the Web App

### Deploy the Web App

```bash
cd examples/web-app
./bundle.sh
cd ../..

./target/release/pied-piper deploy ./examples/web-app/web-app.tar
```

**Save the web app CID!**

### Access in Browser

Open your browser and navigate to:
```
http://localhost:3000/cid/<WEB_APP_CID>/
```

You should see the Pied Piper demo web application with:
- Responsive gradient design
- Interactive buttons
- Network information
- CID display

### Test Static Asset Loading

The browser will automatically load:
- `styles.css` (for styling)
- `app.js` (for interactivity)

Check the browser's Network tab to see:
- Proper Content-Type headers
- Cache-Control headers
- ETag headers

## Troubleshooting

### Issue: Port Already in Use

**Error:** `Address already in use (os error 48)`

**Solution:** Use a different port:
```bash
./target/release/pied-piper gateway --listen 127.0.0.1:3001
```

### Issue: Module Not Found

**Error:** 404 or "Module not found"

**Solutions:**
1. Verify the CID is correct
2. Make sure the gateway is running on the same machine where you deployed
3. Check that the gateway's network node has the module in its cache

### Issue: Gateway Won't Start

**Check:**
```bash
# See if something is using port 3000
lsof -i :3000

# Try a different port
./target/release/pied-piper gateway --listen 127.0.0.1:8888
```

### Issue: WASM Module Build Fails

**Error:** `can't find crate for 'core'`

**Solution:**
```bash
# Install the WASM target
rustup target add wasm32-wasip2

# Rebuild
cd examples/hello-api
cargo build --target wasm32-wasip2 --release
```

### Issue: Empty Response from API

**Check:**
1. Verify the endpoint path is correct (`/api/hello` not `/hello`)
2. Check gateway logs for errors
3. Verify the WASM module was built recently

## Advanced Testing

### Test with Multiple Nodes

Terminal 1 - Deploy Node:
```bash
./target/release/pied-piper deploy module.wasm
# Note the CID and Peer ID
```

Terminal 2 - Gateway Node (with bootstrap):
```bash
./target/release/pied-piper gateway \
  --listen 127.0.0.1:3000 \
  --bootstrap /ip4/127.0.0.1/tcp/<port>/p2p/<peer_id>
```

### Test with Custom Timeout

```bash
./target/release/pied-piper gateway \
  --listen 127.0.0.1:3000 \
  --timeout 60
```

### Enable Verbose Logging

```bash
./target/release/pied-piper gateway \
  --listen 127.0.0.1:3000 \
  --verbose
```

Or set environment variable:
```bash
RUST_LOG=debug ./target/release/pied-piper gateway --listen 127.0.0.1:3000
```

## Performance Testing

### Load Test with curl

```bash
# Simple load test
for i in {1..100}; do
  curl -s "http://localhost:3000/cid/<CID>/api/hello?name=User$i" &
done
wait
```

### Measure Response Time

```bash
time curl "http://localhost:3000/cid/<CID>/api/hello"
```

### Check Gateway Metrics

```bash
# Number of requests
curl http://localhost:3000/cid/<CID>/api/health | jq '.'

# Module cache status
# (feature to be added)
```

## Success Criteria

✅ Module deploys successfully with CID  
✅ Gateway starts without errors  
✅ API endpoints return correct JSON responses  
✅ Web app loads with all assets  
✅ Static files have proper Content-Type  
✅ Caching headers are set correctly  
✅ No errors in gateway logs  

## What We're Testing

This end-to-end test validates:

1. **Module Deployment**
   - WASM compilation
   - Content-addressed storage (CID generation)
   - DHT record creation
   - Local provider registration

2. **HTTP Gateway**
   - HTTP server startup
   - URL routing (by CID)
   - Request handling
   - Error responses

3. **WASM Execution**
   - Module loading from CID
   - WasmRequest serialization to stdin
   - WASM function execution
   - WasmResponse parsing from stdout
   - JSON request/response handling

4. **Static Asset Serving**
   - TAR bundle extraction
   - Content-Type detection
   - Caching headers
   - Multi-file app support

5. **Complete Workflow**
   - Deploy → Store → Serve → Execute → Respond

## Next Steps

After successful testing:

- Deploy more complex applications
- Test multi-module applications
- Add authentication
- Implement WebSocket support
- Create production deployment guides

## References

- [hello-api README](../examples/hello-api/README.md)
- [web-app README](../examples/web-app/README.md)
- [Gateway Documentation](GATEWAY.md)
- [Project Status](STATUS.md)
