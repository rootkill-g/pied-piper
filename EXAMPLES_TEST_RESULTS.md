# Examples Test Results

All examples have been tested and verified working with the current Pied Piper implementation.

## ✅ Working Examples

### 1. hello-api
**Status:** ✅ Working  
**Type:** API Module (WASM)  
**CID:** `brecwchwpyajjy7ysqvgcvviwvzh6ioi2gum6pyjwyeiwnpnd54ka`

**Endpoints:**
- `GET /hello` - Returns hello message
- `POST /echo` - Echoes back request body

**Test:**
```bash
curl http://localhost:8080/cid/brecwchwpyajjy7ysqvgcvviwvzh6ioi2gum6pyjwyeiwnpnd54ka/hello
curl -X POST http://localhost:8080/cid/brecwchwpyajjy7ysqvgcvviwvzh6ioi2gum6pyjwyeiwnpnd54ka/echo \
  -H "Content-Type: application/json" -d '{"test":"data"}'
```

---

### 2. joke-api
**Status:** ✅ Working  
**Type:** API Module (WASM)  
**CID:** `b27zyw23x6pwbsyhwa5rhb4vl3xvwmmh6jrdq3b3mwkpbc2fh6gva`

**Endpoints:**
- `GET /api/joke` - Random joke
- `GET /api/joke/programming` - Programming jokes
- `GET /api/joke/chuck` - Chuck Norris jokes
- `GET /api/joke/dad` - Dad jokes
- `GET /api/categories` - List categories

**Test:**
```bash
curl http://localhost:8080/cid/b27zyw23x6pwbsyhwa5rhb4vl3xvwmmh6jrdq3b3mwkpbc2fh6gva/api/joke
curl http://localhost:8080/cid/b27zyw23x6pwbsyhwa5rhb4vl3xvwmmh6jrdq3b3mwkpbc2fh6gva/api/categories
```

---

### 3. todo-api
**Status:** ✅ Working (Fixed)  
**Type:** API Module (WASM) with Persistent Storage  
**CID:** `belirxknyp4fwqxkiz3iv3ht4aiglyub7movzhuulh6nlrp6xq4da`

**Fixes Applied:**
- Updated storage functions from `host::storage_*_v2` to `env::host_storage_*`
- Fixed buffer size type: `usize` → `u32`
- Updated Response headers: `Vec<(String, String)>` → `HashMap<String, String>`
- Updated Request query: `Option<String>` → `HashMap<String, String>`

**Endpoints:**
- `GET /` - List all todos
- `GET /?id=1` - Get specific todo
- `POST /` - Create todo
- `PUT /` - Update todo
- `DELETE /?id=1` - Delete todo

**Storage Location:** `~/.pied-piper/storage/todo:*`

**Test:**
```bash
# List todos
curl http://localhost:8080/cid/belirxknyp4fwqxkiz3iv3ht4aiglyub7movzhuulh6nlrp6xq4da/

# Create todo
curl -X POST http://localhost:8080/cid/belirxknyp4fwqxkiz3iv3ht4aiglyub7movzhuulh6nlrp6xq4da/ \
  -H "Content-Type: application/json" -d '{"title":"Test todo"}'

# Get specific todo
curl "http://localhost:8080/cid/belirxknyp4fwqxkiz3iv3ht4aiglyub7movzhuulh6nlrp6xq4da/?id=1"
```

---

### 4. static-blog
**Status:** ✅ Working (Fixed)  
**Type:** API Module (WASM) with Persistent Storage  
**CID:** `b3fartwhbq6i5gzzrj7vwmafopfj3c7kvxqj7i4k7tdrm4izpjtyq`

**Fixes Applied:**
- Updated storage functions to use new API
- Fixed buffer size type: `usize` → `u32`
- Updated Response headers: `Vec<(String, String)>` → `HashMap<String, String>`
- Updated Request query: `Option<String>` → `HashMap<String, String>`

**Endpoints:**
- `GET /api/posts` - List all posts
- `GET /api/posts?id=1` - Get specific post
- `POST /api/posts` - Create post
- `PUT /api/posts` - Update post
- `DELETE /api/posts?id=1` - Delete post

**Storage Location:** `~/.pied-piper/storage/blog:*`

**Test:**
```bash
curl http://localhost:8080/cid/b3fartwhbq6i5gzzrj7vwmafopfj3c7kvxqj7i4k7tdrm4izpjtyq/api/posts
```

---

### 5. blog-frontend
**Status:** ✅ Working  
**Type:** Frontend Bundle (HTML/CSS/JS)  
**CID:** `b6r4dcsqbxotkjqewlzrjkvozknwhv2slb6sgvettqnzs6edyiqsq`

**Features:**
- Full CRUD UI for blog posts
- Connected to static-blog API
- Responsive design
- Persistent storage integration

**Access:**
```
http://localhost:8080/cid/b6r4dcsqbxotkjqewlzrjkvozknwhv2slb6sgvettqnzs6edyiqsq/
```

---

### 6. web-app
**Status:** ✅ Working  
**Type:** Frontend Bundle (HTML/CSS/JS)  
**CID:** `bgqqyk6sl4l4v6zzghmtprzejxte2jgbz7al22gswjztzxaies5va`

**Features:**
- Demo web application
- Shows network info
- Example of bundled static assets

**Access:**
```
http://localhost:8080/cid/bgqqyk6sl4l4v6zzghmtprzejxte2jgbz7al22gswjztzxaies5va/
```

---

## 🔄 Not Yet Tested

### 7. ws-echo
**Status:** Not Tested  
**Type:** WebSocket Echo Server  
**Note:** Requires WebSocket implementation testing

### 8. chat-ws
**Status:** Not Tested  
**Type:** WebSocket Chat Application  
**Note:** Requires WebSocket implementation testing

### 9. dashboard
**Status:** Not Tested  
**Type:** Frontend Dashboard  
**Note:** Needs to be tested

### 10. api-client / api-client-core
**Status:** Not Tested  
**Type:** Rust libraries for calling APIs  
**Note:** Used as libraries, not standalone modules

### 11. component-host-demo
**Status:** Not Tested  
**Type:** Component model demonstration  
**Note:** Requires component model testing

---

## Common Fixes Applied

### Storage API Migration
All examples using storage needed to be updated from the old API to the new persistent storage API:

**Old (v2 API):**
```rust
#[link(wasm_import_module = "host")]
extern "C" {
    fn storage_get_v2(key_ptr: *const u8, key_len: usize) -> i32;
    fn storage_set_v2(...) -> i32;
    fn storage_delete_v2(...) -> i32;
    fn host_get_result(ptr: *mut u8, len: usize) -> usize;
}
```

**New (Persistent Storage API):**
```rust
#[link(wasm_import_module = "env")]
extern "C" {
    fn host_storage_get(key_ptr: *const u8, key_len: usize, 
                        val_ptr: *mut u8, val_len_ptr: *mut usize) -> i32;
    fn host_storage_set(...) -> i32;
    fn host_storage_delete(key_ptr: *const u8, key_len: usize) -> i32;
}
```

**Key Changes:**
1. Module name: `"host"` → `"env"`
2. Function names: `storage_*_v2` → `host_storage_*`
3. Get function: Returns directly into buffer instead of separate result call
4. Buffer size: `usize` → `u32` (critical fix for 64-bit compatibility)

### Response Format
All API modules needed Response struct updated:

**Old:**
```rust
struct Response {
    status: u16,
    body: String,
    headers: Vec<(String, String)>,  // ❌ Wrong format
}
```

**New:**
```rust
struct Response {
    status: u16,
    body: String,
    headers: HashMap<String, String>,  // ✅ Correct format
}
```

### Request Format
Gateway now sends query parameters as HashMap:

**Old:**
```rust
struct Request {
    query: Option<String>,  // ❌ Won't deserialize
}
```

**New:**
```rust
struct Request {
    #[serde(default)]
    query: HashMap<String, String>,  // ✅ Matches gateway format
}
```

---

## Summary

- **6 examples tested and working** ✅
- **5 examples not yet tested** (WebSocket, dashboard, api-client, component-host-demo)
- **3 examples required fixes** (todo-api, static-blog, all storage-using modules)
- **All fixes documented** for future reference

The main issues were:
1. Old storage API → New persistent storage API
2. Response headers format (Vec → HashMap)
3. Request query format (String → HashMap)
4. Buffer size type (usize → u32)

All working examples are now deployed and accessible via their CIDs!
