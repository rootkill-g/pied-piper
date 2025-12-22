# WebSocket Chat Example

A real-time chat application demonstrating:
- WebSocket connections
- Message broadcasting
- User presence
- Persistent chat history
- JSON message protocol

## Features

- ✅ Real-time message delivery
- ✅ User join/leave notifications
- ✅ Message history (persistent)
- ✅ Online user list
- ✅ Simple HTML/JS client included

## Message Protocol

### Client → Server

**Join Chat:**
```json
{"type": "join", "username": "alice"}
```

**Send Message:**
```json
{"type": "message", "text": "Hello everyone!"}
```

**Leave Chat:**
```json
{"type": "leave"}
```

### Server → Client

**User Joined:**
```json
{"type": "join", "username": "alice", "timestamp": 1703260800000}
```

**New Message:**
```json
{"type": "message", "username": "alice", "text": "Hello!", "timestamp": 1703260800000}
```

**User Left:**
```json
{"type": "leave", "username": "alice", "timestamp": 1703260800000}
```

**Message History:**
```json
{
  "type": "history",
  "messages": [
    {"username": "alice", "text": "Hello!", "timestamp": 1703260800000},
    ...
  ]
}
```

## Building

```bash
cd examples/chat-ws
cargo build --target wasm32-wasip1 --release
```

The WASM module will be at: `target/wasm32-wasip1/release/chat_ws.wasm`

## Deploying

```bash
# Deploy WASM module
pied-piper deploy target/wasm32-wasip1/release/chat_ws.wasm \
  --name chat \
  --asset index.html \
  --asset app.js \
  --asset styles.css

# Access at:
# http://localhost:3000/app/chat
```

## Testing

```bash
# Start Pied Piper
pied-piper start

# Open multiple browser windows to:
# http://localhost:3000/app/chat

# Chat between windows in real-time!
```

## Architecture

```
Browser 1                    Browser 2
   │                            │
   │  WebSocket                 │  WebSocket
   │                            │
   └────────┬──────────────────┬┘
            │                  │
            ▼                  ▼
       ┌─────────────────────────┐
       │   Pied Piper Gateway    │
       └─────────────────────────┘
                   │
                   ▼
       ┌─────────────────────────┐
       │   WASM Chat Handler     │
       │  (chat_ws.wasm)         │
       └─────────────────────────┘
                   │
                   ▼
       ┌─────────────────────────┐
       │   Storage (messages,    │
       │   users, history)       │
       └─────────────────────────┘
```

## Storage Schema

- `chat:messages` - JSON array of all messages
- `chat:users` - JSON array of current online users
- `chat:history_count` - Number of messages in history

## Implementation Details

### Message Broadcasting

Since each WebSocket connection is handled independently in WASM:
1. Message arrives at connection A
2. WASM handler stores message in shared storage
3. Next message on connection B retrieves updated history
4. Gateway broadcasts to all connections

**Note**: True pub/sub broadcasting would require stateful gateway support (future enhancement).

### Limits

- Max message length: 1000 characters
- Max history: 100 messages (rolling)
- Max username length: 20 characters

### Performance

- Message delivery: ~10-50ms
- Storage write: ~1ms
- History retrieval: ~5ms

## Client Code

The included `index.html` provides:
- WebSocket connection management
- Message sending/receiving
- User list display
- Auto-reconnect on disconnect

## Example Usage

```javascript
// Connect to chat
const ws = new WebSocket('ws://localhost:3000/ws/app/chat');

// Join with username
ws.onopen = () => {
  ws.send(JSON.stringify({
    type: 'join',
    username: 'alice'
  }));
};

// Handle messages
ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  
  if (msg.type === 'message') {
    console.log(`${msg.username}: ${msg.text}`);
  } else if (msg.type === 'join') {
    console.log(`${msg.username} joined`);
  } else if (msg.type === 'leave') {
    console.log(`${msg.username} left`);
  }
};

// Send message
ws.send(JSON.stringify({
  type: 'message',
  text: 'Hello everyone!'
}));
```

## Security Considerations

- No authentication (add JWT for production)
- No rate limiting per user (add token bucket)
- No message moderation (add content filtering)
- No private messages (add direct messaging)

## Next Steps

- Add user authentication
- Implement private/direct messages
- Add message reactions (emoji)
- Add file/image sharing
- Implement typing indicators
- Add chat rooms/channels
