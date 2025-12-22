#!/usr/bin/env bash

# Bundle web app into TAR archive for deployment

set -e

echo "📦 Bundling web app..."

# Remove old bundle if exists
rm -f web-app.tar

# Create TAR archive with all assets
tar -cf web-app.tar index.html styles.css app.js

echo "✅ Bundle created: web-app.tar"
echo ""
echo "Contents:"
tar -tf web-app.tar

echo ""
echo "Bundle size: $(du -h web-app.tar | cut -f1)"
echo ""
echo "To deploy:"
echo "  ../../target/release/pied-piper deploy web-app.tar"
