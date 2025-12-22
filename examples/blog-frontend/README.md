# Blog Frontend

A beautiful, responsive web frontend for the static-blog API with persistent storage.

## Features

- 📝 **Create** new blog posts
- 📖 **Read** all posts with newest first
- ✏️ **Edit** existing posts
- 🗑️ **Delete** posts
- 💾 **Persistent Storage** - Data survives gateway restarts
- 🎨 **Beautiful UI** - Gradient design with smooth animations
- 📱 **Responsive** - Works on desktop and mobile

## Quick Start

### 1. Build and Deploy the API (if not already done)

```bash
# Build the API
cd ../static-blog
cargo build --target wasm32-wasip1 --release

# Deploy it
cd ../../
./target/release/pied-piper deploy \
  examples/static-blog/target/wasm32-wasip1/release/static-blog.wasm \
  --name static-blog \
  --version 0.1.0

# Note the CID returned (e.g., b3fartwhbq6i5gzzrj7vwmafopfj3c7kvxqj7i4k7tdrm4izpjtyq)
```

### 2. Bundle and Deploy the Frontend

```bash
cd examples/blog-frontend

# Create bundle
./bundle.sh

# Deploy bundle
../../target/release/pied-piper deploy --name blog-frontend blog-frontend.tar
```

### 3. Access in Browser

Open the URL provided after deployment:
```
http://localhost:8080/cid/<FRONTEND_CID>/
```

Current deployment:
```
Frontend: http://localhost:8080/cid/b6r4dcsqbxotkjqewlzrjkvozknwhv2slb6sgvettqnzs6edyiqsq/
API CID: b3fartwhbq6i5gzzrj7vwmafopfj3c7kvxqj7i4k7tdrm4izpjtyq
```

## How It Works

### Architecture

```
Browser
   ↓ (HTTP)
Gateway (localhost:8080)
   ↓
   ├─→ Frontend Bundle (HTML/CSS/JS)
   └─→ API Module (WASM)
          ↓
       Persistent Storage (~/.pied-piper/storage/)
```

### API Integration

The frontend automatically detects the gateway URL and connects to the API:

```javascript
// Frontend calls API on the same gateway
const apiUrl = `${window.location.origin}/cid/${API_CID}/api/posts`;
```

### Storage Location

All blog posts are stored persistently at:
```
~/.pied-piper/storage/
├── 626c6f673a6e6578745f6964.dat      (blog:next_id)
├── 626c6f673a706f73743a31.dat        (blog:post:1)
├── 626c6f673a706f73743a32.dat        (blog:post:2)
└── 626c6f673a706f737473.dat          (blog:posts index)
```

Files are hex-encoded and survive gateway restarts!

## Configuration

You can change the API CID directly in the UI:

1. Scroll to "API Configuration" section
2. Enter your API module CID
3. The frontend will automatically reload posts

## API Endpoints Used

- `GET /api/posts` - List all posts
- `GET /api/posts?id=1` - Get single post
- `POST /api/posts` - Create new post
- `PUT /api/posts` - Update existing post
- `DELETE /api/posts?id=1` - Delete post

## Development

### Modify and Redeploy

```bash
# 1. Edit files (index.html, styles.css, app.js)
nano index.html

# 2. Rebuild bundle
./bundle.sh

# 3. Redeploy
../../target/release/pied-piper deploy --name blog-frontend blog-frontend.tar

# 4. Open new CID in browser
```

### Test Locally (without deployment)

You can also serve the files locally for development:

```bash
# Start a local server
python3 -m http.server 3000

# Open http://localhost:3000
# Update API CID in the configuration section
```

## Features Demo

### Create Post
1. Fill in title and content
2. Click "Create Post"
3. Post appears in the list below

### Edit Post
1. Click "✏️ Edit" on any post
2. Modify title/content in modal
3. Click "Update Post"
4. Changes are saved

### Delete Post
1. Click "🗑️ Delete" on any post
2. Confirm deletion
3. Post is removed

### Persistence Test
1. Create some posts
2. Restart the gateway: `pkill -f "pied-piper gateway" && pied-piper gateway --listen 0.0.0.0:8080`
3. Refresh the page
4. All posts are still there! ✅

## Troubleshooting

### "Error loading posts"
- Check that the gateway is running: `ps aux | grep pied-piper`
- Verify the API CID is correct
- Check gateway logs: `tail -f /tmp/gateway.log`

### Empty posts list but posts were created
- This was due to a buffer size type mismatch (now fixed)
- Make sure you're using API version 0.1.1 or later

### Browser console errors
- Check CORS settings (should work since same-origin)
- Verify the API module is deployed correctly
- Try accessing API directly: `curl http://localhost:8080/cid/<API_CID>/api/posts`

## Tech Stack

- **Frontend**: Vanilla JavaScript (no frameworks!)
- **Styling**: Pure CSS with gradients and animations
- **API**: Rust WASM module with persistent storage
- **Storage**: File-based key-value store
- **Network**: Pied Piper P2P network with HTTP gateway

## License

MIT - Part of the Pied Piper project
