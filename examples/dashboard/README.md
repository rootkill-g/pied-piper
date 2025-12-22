# Dashboard - Interactive Frontend

An interactive web dashboard for managing and testing Pied Piper modules.

## Features

- **Auto-Discovery**: Automatically discovers available modules
- **Live Testing**: Test any endpoint directly from the browser
- **Real-time Stats**: See module count and endpoint statistics
- **Manual Add**: Add modules by CID
- **Beautiful UI**: Modern, responsive design
- **Interactive**: Click to test any API endpoint

## Build

```bash
cargo build --target wasm32-wasip2 --release
```

## Deploy

```bash
cd /Users/rootkill/pied-piper
./target/release/pied-piper deploy examples/dashboard/target/wasm32-wasip2/release/dashboard.wasm
```

## Access

After deployment, access the dashboard at:
```
http://localhost:8080/cid/<DASHBOARD_CID>/
```

The dashboard will automatically:
1. Discover available modules (hello-api, joke-api)
2. Fetch their `/api/info` endpoints
3. Display all endpoints
4. Allow you to test them with one click

## Features

### Module Discovery
- Auto-discovers known modules
- Fetches module metadata from `/api/info`
- Shows all available endpoints

### Interactive Testing
- Click "Test API" button on any endpoint
- See live responses
- JSON formatting
- Error handling

### Manual Module Addition
- Enter any CID to add a module
- Fetches info automatically
- Persists in session

## Technical Details

The dashboard is itself a WASM module that:
1. Serves a single-page HTML application
2. The frontend makes fetch calls to `/cid/<CID>/api/*`
3. Works entirely client-side
4. No backend database needed - discovers modules dynamically
