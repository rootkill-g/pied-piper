# Web App Example

This is a complete web application that demonstrates the Pied Piper platform's ability to serve bundled frontend assets.

## Structure

```
web-app/
├── index.html    # Main HTML page
├── styles.css    # Styling
├── app.js        # JavaScript logic
├── bundle.sh     # Script to create TAR bundle
└── README.md     # This file
```

## Features

- ✅ Complete HTML/CSS/JS application
- ✅ TAR bundle format for deployment
- ✅ Content-addressed distribution
- ✅ Responsive design
- ✅ Interactive UI

## Building & Deploying

### 1. Create the TAR bundle

```bash
cd examples/web-app
./bundle.sh
```

This creates `web-app.tar` containing all assets.

### 2. Deploy to Pied Piper network

```bash
# From the project root
./target/release/pied-piper deploy examples/web-app/web-app.tar
```

This will output a CID (Content Identifier) for your application.

### 3. Access via HTTP Gateway

```bash
# Start the gateway (if not already running)
./target/release/pied-piper gateway --port 8080

# Access your app
open http://localhost:8080/cid/<your-cid>/
```

## How It Works

1. **Bundling**: All assets (HTML, CSS, JS) are packaged into a TAR archive
2. **Deployment**: The bundle is uploaded to the P2P network with a unique CID
3. **Distribution**: The gateway serves files from the bundle based on URL paths
4. **Routing**: 
   - `/cid/<cid>/` → serves `index.html`
   - `/cid/<cid>/styles.css` → serves `styles.css`
   - `/cid/<cid>/app.js` → serves `app.js`

## Content Types

The gateway automatically detects content types based on file extensions:

- `.html` → `text/html`
- `.css` → `text/css`
- `.js` → `application/javascript`
- `.json` → `application/json`
- `.png`, `.jpg`, `.svg` → appropriate image types
- `.woff`, `.woff2`, `.ttf` → font types

## Caching

The gateway sets appropriate cache headers:

- **Static assets**: `Cache-Control: public, max-age=31536000, immutable`
- **ETag**: Based on Blake3 hash of content
- **Immutable**: Content-addressed files never change

## Future Enhancements

- [ ] WebAssembly module integration
- [ ] Service Worker for offline support
- [ ] Progressive Web App (PWA) capabilities
- [ ] Hot module replacement for development
- [ ] Asset optimization (minification, compression)

## Testing

After deployment, you can test:

1. Homepage loads correctly
2. CSS styling is applied
3. JavaScript executes
4. Assets are cached properly
5. ETag headers are present

## Notes

- This is a frontend-only example
- For backend logic, see `examples/hello-api`
- Bundle size should be kept reasonable (<10MB recommended)
- All assets must be included in the TAR archive
