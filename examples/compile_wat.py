#!/usr/bin/env python3
"""
Compile WAT to WASM using Python's wasmtime package
Install with: pip install wasmtime
"""
import sys

try:
    from wasmtime import wat2wasm
except ImportError:
    print("Error: wasmtime package not installed")
    print("Install with: pip install wasmtime")
    sys.exit(1)

# Read the WAT file
with open('examples/hello.wat', 'r') as f:
    wat_content = f.read()

# Convert to WASM
wasm_bytes = wat2wasm(wat_content)

# Write the WASM file
with open('examples/hello.wasm', 'wb') as f:
    f.write(wasm_bytes)

print(f"Successfully compiled hello.wat to hello.wasm ({len(wasm_bytes)} bytes)")
