# TAR Bundles

Frontend applications packaged as TAR archives (HTML/CSS/JS)

## Examples

- **blog-frontend** - Complete blog UI ✅
- **web-app** - Demo frontend with asset serving ✅

## Build

```bash
cd <example-name>
./bundle.sh
```

Output: `<example-name>.tar`

## Deploy

```bash
../../target/release/pied-piper deploy --name <name> <example-name>.tar
```

## Access

Frontend bundles are accessed via browser:
```
http://localhost:8080/cid/<CID>/
http://localhost:8080/app/<name>/
```

**Note**: Trailing slash is required for proper relative path resolution.

## Features

- Automatic content-type detection
- CSS, JavaScript, image serving
- Index.html as default route
- 301 redirect to add trailing slash if missing

See [main examples README](../README.md) for detailed documentation.
