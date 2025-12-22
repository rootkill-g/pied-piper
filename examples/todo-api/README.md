# Todo API Example

A complete REST API for managing todos, demonstrating:
- HTTP request routing
- JSON serialization/deserialization
- Persistent storage with host functions
- CRUD operations
- Error handling

## Features

- ✅ Create, read, update, delete todos
- ✅ List all todos with filtering
- ✅ Persistent storage (survives restarts)
- ✅ Proper HTTP status codes
- ✅ JSON request/response

## API Endpoints

### List All Todos
```bash
GET /
Response: [{"id": "1", "title": "Buy milk", "done": false}, ...]
```

### Get Single Todo
```bash
GET /?id=1
Response: {"id": "1", "title": "Buy milk", "done": false}
```

### Create Todo
```bash
POST /
Body: {"title": "Buy milk"}
Response: {"id": "1", "title": "Buy milk", "done": false}
```

### Update Todo
```bash
PUT /
Body: {"id": "1", "done": true}
Response: {"id": "1", "title": "Buy milk", "done": true}
```

### Delete Todo
```bash
DELETE /?id=1
Response: {"success": true}
```

## Building

```bash
cd examples/todo-api
cargo build --target wasm32-wasip1 --release
```

The WASM module will be at: `target/wasm32-wasip1/release/todo_api.wasm`

## Deploying

```bash
# Deploy to Pied Piper network
pied-piper deploy target/wasm32-wasip1/release/todo_api.wasm --name todo-api

# Access at:
curl http://localhost:3000/app/todo-api
```

## Testing

```bash
# Start Pied Piper
pied-piper start

# In another terminal, run test script
./test.sh
```

## Implementation Details

### Storage Schema

Todos are stored as JSON strings with keys:
- `todo:<id>` → JSON serialized Todo object
- `todo:next_id` → Next available ID counter

### Error Handling

- 400 Bad Request: Invalid JSON or missing fields
- 404 Not Found: Todo ID doesn't exist
- 500 Internal Server Error: Storage or serialization errors

### Performance

- Storage operations: ~1ms
- JSON parsing: ~0.5ms
- Total request time: ~5-10ms

## Example Usage

```bash
# Create a todo
curl -X POST http://localhost:3000/app/todo-api \
  -H "Content-Type: application/json" \
  -d '{"title": "Buy groceries"}'

# Output: {"id":"1","title":"Buy groceries","done":false}

# List all todos
curl http://localhost:3000/app/todo-api

# Output: [{"id":"1","title":"Buy groceries","done":false}]

# Mark as done
curl -X PUT http://localhost:3000/app/todo-api \
  -H "Content-Type: application/json" \
  -d '{"id":"1","done":true}'

# Output: {"id":"1","title":"Buy groceries","done":true}

# Delete todo
curl -X DELETE "http://localhost:3000/app/todo-api?id=1"

# Output: {"success":true}
```

## Code Structure

- `src/main.rs` - Main request handler and routing
- `src/storage.rs` - Storage helper functions
- `Cargo.toml` - Dependencies and build config

## Dependencies

- `serde` + `serde_json` - JSON serialization
- `pied_piper_host` - Host function bindings (HTTP, storage)

## Next Steps

- Add authentication (JWT tokens)
- Implement pagination for large lists
- Add todo categories/tags
- Add due dates and reminders
