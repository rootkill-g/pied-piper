#!/bin/bash
set -e

BASE_URL="http://localhost:3000/app/todo-api"

echo "🧪 Testing Todo API..."
echo

# Test 1: Create a todo
echo "1️⃣  Creating todo..."
TODO1=$(curl -s -X POST "$BASE_URL" \
  -H "Content-Type: application/json" \
  -d '{"title":"Buy groceries"}')
echo "Created: $TODO1"
ID1=$(echo $TODO1 | jq -r '.id')
echo

# Test 2: Create another todo
echo "2️⃣  Creating another todo..."
TODO2=$(curl -s -X POST "$BASE_URL" \
  -H "Content-Type: application/json" \
  -d '{"title":"Walk the dog"}')
echo "Created: $TODO2"
ID2=$(echo $TODO2 | jq -r '.id')
echo

# Test 3: List all todos
echo "3️⃣  Listing all todos..."
curl -s "$BASE_URL" | jq '.'
echo

# Test 4: Get specific todo
echo "4️⃣  Getting todo $ID1..."
curl -s "$BASE_URL?id=$ID1" | jq '.'
echo

# Test 5: Update todo (mark as done)
echo "5️⃣  Marking todo $ID1 as done..."
curl -s -X PUT "$BASE_URL" \
  -H "Content-Type: application/json" \
  -d "{\"id\":\"$ID1\",\"done\":true}" | jq '.'
echo

# Test 6: Update todo (change title)
echo "6️⃣  Updating todo $ID2 title..."
curl -s -X PUT "$BASE_URL" \
  -H "Content-Type: application/json" \
  -d "{\"id\":\"$ID2\",\"title\":\"Walk the dog (urgent!)\"}" | jq '.'
echo

# Test 7: List todos again (should show updates)
echo "7️⃣  Listing todos (after updates)..."
curl -s "$BASE_URL" | jq '.'
echo

# Test 8: Delete todo
echo "8️⃣  Deleting todo $ID1..."
curl -s -X DELETE "$BASE_URL?id=$ID1" | jq '.'
echo

# Test 9: Verify deletion
echo "9️⃣  Listing todos (after deletion)..."
curl -s "$BASE_URL" | jq '.'
echo

# Test 10: Try to get deleted todo (should fail)
echo "🔟 Trying to get deleted todo (should 404)..."
curl -s "$BASE_URL?id=$ID1" | jq '.'
echo

echo "✅ All tests complete!"
