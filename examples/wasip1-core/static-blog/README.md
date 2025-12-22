# Static Blog Example

A complete static blog with dynamic backend demonstrating:
- Static HTML/CSS/JS frontend
- Dynamic WASM backend (API)
- Asset bundling and serving
- Persistent storage
- SPA routing

## Features

- ✅ List blog posts
- ✅ View individual posts
- ✅ Create new posts
- ✅ Edit existing posts
- ✅ Delete posts
- ✅ Markdown rendering (client-side)
- ✅ Responsive design
- ✅ Single Page Application (SPA)

## Architecture

```
Static Frontend (HTML/CSS/JS)
         │
         │  AJAX requests
         ▼
  WASM Blog API (/api/*)
         │
         ▼
  Persistent Storage
```

## API Endpoints

### List Posts
```bash
GET /api/posts
Response: [{"id":"1","title":"Hello","content":"...","created_at":1703260800000}, ...]
```

### Get Single Post
```bash
GET /api/posts?id=1
Response: {"id":"1","title":"Hello","content":"...","created_at":1703260800000}
```

### Create Post
```bash
POST /api/posts
Body: {"title":"Hello","content":"My first post"}
Response: {"id":"1","title":"Hello","content":"...","created_at":1703260800000}
```

### Update Post
```bash
PUT /api/posts
Body: {"id":"1","title":"Updated","content":"New content"}
Response: {"id":"1","title":"Updated","content":"...","updated_at":1703260800000}
```

### Delete Post
```bash
DELETE /api/posts?id=1
Response: {"success":true}
```

## Building

```bash
cd examples/static-blog
cargo build --target wasm32-wasip1 --release
```

The WASM module will be at: `target/wasm32-wasip1/release/static_blog.wasm`

## Deploying

```bash
# Deploy with all assets
pied-piper deploy target/wasm32-wasip1/release/static_blog.wasm \
  --name blog \
  --asset index.html \
  --asset app.js \
  --asset styles.css \
  --asset marked.min.js

# Access at:
# http://localhost:3000/app/blog
```

## Project Structure

```
static-blog/
├── Cargo.toml
├── README.md
├── src/
│   └── main.rs          # WASM backend API
├── index.html           # Frontend HTML
├── app.js               # Frontend JavaScript
├── styles.css           # Frontend styles
└── marked.min.js        # Markdown parser
```

## Frontend Features

### Routing
- `/` - Post list
- `/post/:id` - View single post
- `/new` - Create new post
- `/edit/:id` - Edit post

### Markdown Support
Uses marked.js for client-side Markdown rendering:
- Headers, lists, code blocks
- Links and images
- Bold, italic, code

### Responsive Design
- Mobile-friendly layout
- Touch-optimized
- Adapts to screen size

## Storage Schema

- `blog:posts` - JSON array of all post IDs
- `blog:post:<id>` - JSON object for individual post
- `blog:next_id` - Next available post ID

## Implementation Details

### Post Structure
```rust
struct Post {
    id: String,
    title: String,
    content: String,        // Markdown format
    created_at: u64,        // Unix timestamp
    updated_at: Option<u64> // Unix timestamp
}
```

### API Routing
The backend routes requests based on path:
- `/api/posts` → Post API handlers
- `/*` → Static asset serving (HTML, CSS, JS)

### SPA Fallback
All non-API routes serve `index.html` for client-side routing.

## Example Usage

```bash
# Start Pied Piper
pied-piper start

# Open in browser
open http://localhost:3000/app/blog

# Use the UI to:
# 1. Create a new post with Markdown content
# 2. View the rendered post
# 3. Edit or delete posts
```

## Testing with cURL

```bash
# Create a post
curl -X POST http://localhost:3000/app/blog/api/posts \
  -H "Content-Type: application/json" \
  -d '{
    "title": "My First Post",
    "content": "# Hello World\n\nThis is **markdown**!"
  }'

# List all posts
curl http://localhost:3000/app/blog/api/posts | jq '.'

# Get specific post
curl "http://localhost:3000/app/blog/api/posts?id=1" | jq '.'

# Update post
curl -X PUT http://localhost:3000/app/blog/api/posts \
  -H "Content-Type: application/json" \
  -d '{
    "id": "1",
    "title": "Updated Title",
    "content": "Updated content"
  }'

# Delete post
curl -X DELETE "http://localhost:3000/app/blog/api/posts?id=1"
```

## Customization

### Styling
Edit `styles.css` to change:
- Colors and themes
- Fonts
- Layout and spacing

### Markdown Rendering
Customize marked.js options in `app.js`:
```javascript
marked.setOptions({
  breaks: true,
  gfm: true,
  // ... more options
});
```

### Post Limits
Edit constants in `src/main.rs`:
```rust
const MAX_TITLE_LEN: usize = 200;
const MAX_CONTENT_LEN: usize = 50_000;
```

## Performance

- Initial load: ~100ms
- Post list: ~5ms
- Single post: ~3ms
- Create/Update: ~10ms
- Client-side routing: instant

## Security Considerations

- **No authentication**: Add JWT auth for production
- **No input sanitization**: Markdown XSS protection needed
- **No rate limiting**: Add per-user rate limits
- **Public write access**: Add authorization

## Next Steps

- Add user authentication
- Implement comments
- Add tags/categories
- Full-text search
- RSS feed generation
- Image uploads
- Post scheduling
- Draft/publish workflow
