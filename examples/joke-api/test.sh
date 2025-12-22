#!/bin/bash

# Test script for Joke API
CID="bmjncyyz5pox4zbfwajqib35znicam5q45cxvq4wdvrppd3gv2fra"
BASE_URL="http://localhost:8080/cid/$CID"

echo "🎭 Testing Joke API"
echo "=================="
echo ""

echo "1. Health Check:"
curl -s "$BASE_URL/api/health" | json_pp
echo -e "\n"

echo "2. Random Joke:"
curl -s "$BASE_URL/api/joke" | json_pp
echo -e "\n"

echo "3. Programming Joke:"
curl -s "$BASE_URL/api/joke/programming" | json_pp
echo -e "\n"

echo "4. Chuck Norris Joke:"
curl -s "$BASE_URL/api/joke/chuck" | json_pp
echo -e "\n"

echo "5. Dad Joke:"
curl -s "$BASE_URL/api/joke/dad" | json_pp
echo -e "\n"

echo "6. Categories:"
curl -s "$BASE_URL/api/categories" | json_pp
echo -e "\n"

echo "7. API Info:"
curl -s "$BASE_URL/api/info" | json_pp
