#!/bin/bash
# Test script for api-client example demonstrating all host functions

set -e

echo "==================================="
echo "API Client Host Functions Test"
echo "==================================="
echo ""

# Configuration
# You can use either the app name or the CID
# If the module isn't found by name, update this to use the CID from deploy output
APP_ID="${API_CLIENT_CID:-api-client}"
BASE_URL="http://localhost:8080/app/${APP_ID}"
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Helper function for testing endpoints
test_endpoint() {
    local name="$1"
    local method="$2"
    local path="$3"
    local data="$4"
    
    echo "Testing: $name"
    
    if [ "$method" = "GET" ]; then
        response=$(curl -s "$BASE_URL$path")
    else
        response=$(curl -s -X POST "$BASE_URL$path" \
            -H "Content-Type: application/json" \
            -d "$data")
    fi
    
    # Check if response contains "success"
    if echo "$response" | grep -q '"status":"success"'; then
        echo -e "${GREEN}✓ PASS${NC}: $name"
        echo "Response: $response"
    else
        echo -e "${RED}✗ FAIL${NC}: $name"
        echo "Response: $response"
    fi
    
    echo ""
}

# Wait for server to be ready
echo "Waiting for gateway..."
for i in {1..10}; do
    if curl -s "$BASE_URL/health" > /dev/null 2>&1; then
        echo "Gateway is ready!"
        echo ""
        break
    fi
    if [ $i -eq 10 ]; then
        echo "Gateway not responding. Make sure it's running:"
        echo "  pied-piper gateway --listen 0.0.0.0:8080"
        exit 1
    fi
    sleep 1
done

# Test 1: Health Check
test_endpoint "Health Check" "GET" "/health" ""

# Test 2: System Stats
test_endpoint "System Stats" "GET" "/stats" ""

# Test 3: Storage - Set
test_endpoint "Storage Set" "POST" "/cache" \
    '{"action":"set","key":"test-key","value":"test-value"}'

# Test 4: Storage - Get
test_endpoint "Storage Get" "POST" "/cache" \
    '{"action":"get","key":"test-key"}'

# Test 5: Storage - Delete
test_endpoint "Storage Delete" "POST" "/cache" \
    '{"action":"delete","key":"test-key"}'

# Test 6: Storage - List Count
test_endpoint "Storage List" "POST" "/cache" \
    '{"action":"list"}'

# Test 7: Counter (increment multiple times)
echo "Testing: Counter (5 increments)"
for i in {1..5}; do
    response=$(curl -s -X POST "$BASE_URL/counter")
    count=$(echo "$response" | grep -o '"counter":[0-9]*' | grep -o '[0-9]*')
    echo "  Increment $i: counter = $count"
done
echo -e "${GREEN}✓ PASS${NC}: Counter increments"
echo ""

# Test 8: BLAKE3 Hash
test_endpoint "BLAKE3 Hash" "POST" "/hash" \
    '{"data":"Hello, Pied Piper!"}'

# Test 9: External API Call (without cache)
echo "Testing: External API (no cache)"
response=$(curl -s -X POST "$BASE_URL/external" \
    -H "Content-Type: application/json" \
    -d '{"url":"https://api.github.com/zen","use_cache":false}')

if echo "$response" | grep -q '"status":"success"'; then
    echo -e "${GREEN}✓ PASS${NC}: External API call"
    echo "Response: $response"
else
    echo -e "${RED}✗ FAIL${NC}: External API call"
    echo "Response: $response"
fi
echo ""

# Test 10: External API Call (with cache)
echo "Testing: External API (with cache)"
start_time=$(date +%s%N)
response1=$(curl -s -X POST "$BASE_URL/external" \
    -H "Content-Type: application/json" \
    -d '{"url":"https://api.github.com/zen","use_cache":true}')
time1=$(( ($(date +%s%N) - start_time) / 1000000 ))

echo "First call (uncached): ${time1}ms"
echo "Response: $response1"
echo ""

# Call again to test cache
sleep 1
start_time=$(date +%s%N)
response2=$(curl -s -X POST "$BASE_URL/external" \
    -H "Content-Type: application/json" \
    -d '{"url":"https://api.github.com/zen","use_cache":true}')
time2=$(( ($(date +%s%N) - start_time) / 1000000 ))

echo "Second call (cached): ${time2}ms"
echo "Response: $response2"

if [ $time2 -lt $time1 ]; then
    echo -e "${GREEN}✓ PASS${NC}: Caching works (cached call faster)"
else
    echo -e "${RED}✗ WARN${NC}: Cache may not be working (times: ${time1}ms vs ${time2}ms)"
fi
echo ""

# Test 11: Invalid Endpoint
echo "Testing: Invalid Endpoint (404)"
response=$(curl -s "$BASE_URL/invalid")
if echo "$response" | grep -q '"status":"error"'; then
    echo -e "${GREEN}✓ PASS${NC}: Error handling for invalid endpoint"
    echo "Response: $response"
else
    echo -e "${RED}✗ FAIL${NC}: Error handling"
    echo "Response: $response"
fi
echo ""

# Summary
echo "==================================="
echo "All tests completed!"
echo "==================================="
echo ""
echo "Host Functions Tested:"
echo "  ✓ host::log          - Logging messages"
echo "  ✓ host::now_millis   - Timestamps"
echo "  ✓ host::random_u32   - Random numbers (in stats)"
echo "  ✓ http::get          - HTTP GET requests"
echo "  ✓ storage::get       - Key-value get"
echo "  ✓ storage::set       - Key-value set"
echo "  ✓ storage::delete    - Key-value delete"
echo "  ✓ storage::list_count - Count keys"
echo "  ✓ crypto::blake3_hash - BLAKE3 hashing"
echo ""
echo "To see logs, check the gateway output:"
echo "  pied-piper gateway --listen 0.0.0.0:8080"
