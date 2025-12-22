#!/usr/bin/env bash

# Bundle blog frontend into TAR archive for deployment

set -e

cd "$(dirname "$0")"

echo "📦 Bundling blog frontend..."

# Remove old bundle if exists
rm -f blog-frontend.tar

# Create TAR archive with all assets
tar -cf blog-frontend.tar index.html styles.css app.js

echo "✅ Bundle created: blog-frontend.tar"
echo ""
echo "Contents:"
tar -tf blog-frontend.tar

echo ""
echo "Bundle size: $(du -h blog-frontend.tar | cut -f1)"
echo ""
echo "To deploy:"
echo "  ../../target/release/pied-piper deploy --name blog-frontend blog-frontend.tar"
