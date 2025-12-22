# Production-Ready Routing Implementation

## Overview

The Pied Piper gateway now has production-ready routing with comprehensive security validations, path normalization, and error handling.

## Features Implemented

### 1. CID Validation (`CIDValidator`)
- **Format validation**: Ensures CIDs start with 'b' (base32 encoding)
- **Length checks**: Minimum 30 characters, maximum 100 (prevents DoS)
- **Character validation**: Only allows lowercase a-z and digits 2-7 (base32 alphabet)
- **Early rejection**: Invalid CIDs are rejected before any resource lookup

**Examples:**
```
✅ Valid: brecwchwpyajjy7ysqvgcvviwvzh6ioi2gum6pyjwyeiwnpnd54ka
❌ Invalid: abc123 (too short)
❌ Invalid: Bafybeig... (uppercase not allowed)
❌ Invalid: bafybeig_test (invalid characters)
```

### 2. Path Normalization (`PathSanitizer`)
- **URL decoding**: Properly handles %20, %2F, and other encoded characters
- **Directory traversal protection**: Blocks `../`, `./`, and similar patterns
- **Null byte protection**: Rejects paths containing `\0`
- **Hidden file blocking**: Prevents access to `.htaccess`, `.env`, `.git/`, etc.
- **Path depth limiting**: Maximum 10 levels deep (prevents deeply nested attacks)
- **Special character filtering**: Only allows alphanumeric, `/`, `-`, `_`, `.`
- **System path blocking**: Blocks `/etc/`, `/proc/`, `/sys/`, `/var/`, etc.

**Examples:**
```
✅ Valid: /api/users → api/users
✅ Valid: //api///users// → api/users
✅ Valid: /hello%20world.txt → hello world.txt
❌ Invalid: /../etc/passwd (directory traversal)
❌ Invalid: /.env (hidden file)
❌ Invalid: /path/with/null\0byte (null byte)
❌ Invalid: /a/b/c/d/e/f/g/h/i/j/k/l (too deep)
```

### 3. HTTP Method Validation (`MethodValidator`)
- **Route-specific rules**: Different methods allowed for different route types
- **CID routes without path**: Only GET, HEAD, OPTIONS (for bundles/frontends)
- **CID routes with path**: GET, HEAD, POST, PUT, DELETE, PATCH, OPTIONS (for APIs and assets)
- **App routes**: All standard HTTP methods allowed
- **Proper error responses**: Returns 405 Method Not Allowed with `Allow` header

**Examples:**
```
✅ GET  /cid/xyz/              (valid - bundle access)
✅ GET  /cid/xyz/api/users     (valid - API call)
✅ POST /cid/xyz/api/users     (valid - API call)
❌ POST /cid/xyz/              (invalid - POST not allowed without path)
✅ GET  /app/myapp/api         (valid)
```

### 4. File Extension Validation (`ExtensionValidator`)
- **Allowed extensions**: Web assets (html, css, js, wasm, png, jpg, etc.)
- **Blocked extensions**: Executable/script files (php, exe, sh, dll, bat, etc.)
- **No extension allowed**: API endpoints without extensions pass through
- **Returns 403 Forbidden**: For suspicious file types

**Allowed extensions:**
```
html, htm, css, js, json, wasm, wat
png, jpg, jpeg, gif, svg, ico, webp
woff, woff2, ttf, eot
txt, xml, pdf
mp4, webm, ogg, mp3, wav
zip, tar, gz
```

**Blocked extensions:**
```
php, asp, aspx, jsp, cgi, pl, py, rb
sh, bash, exe, dll, so, dylib
bat, cmd, vbs, ps1
```

### 5. Comprehensive Error Handling
- **Specific error types**: InvalidCID, InvalidPath, MethodNotAllowed, InvalidEncoding
- **Proper HTTP status codes**: 400, 403, 405 for different error types
- **Descriptive error messages**: Clear feedback for debugging
- **Security logging**: All rejections logged with details

## Integration Points

### Server Routes (`src/gateway/server.rs`)

All route handlers now validate before processing:

1. **`handle_cid_request`**: CID validation + method validation
2. **`handle_cid_request_with_path`**: CID + path + extension + method validation
3. **`handle_app_request`**: App name + method validation
4. **`handle_app_request_with_path`**: App name + path + extension + method validation

### Bundle Redirect Logic

The trailing slash redirect for bundles remains intact and works with validation:
- Validates CID before checking if it's a bundle
- Only redirects for GET requests
- Preserves query parameters in redirect

## Testing Results

All security validations tested and working:

```bash
✅ Test 1: Valid CID                    → 200 OK
✅ Test 2: Invalid CID (too short)      → 400 Bad Request
✅ Test 3: Directory traversal attempt  → 403 Forbidden
✅ Test 4: Suspicious file extension    → 403 Forbidden
✅ Test 5: Invalid HTTP method          → 405 Method Not Allowed
✅ Test 6: Valid static asset           → 200 OK
✅ Test 7: URL encoded path             → 200 OK (decoded properly)
```

## Security Benefits

### Protection Against:

1. **Path Traversal Attacks**: `../`, `./`, encoded variations
2. **Code Injection**: PHP, ASP, shell scripts blocked
3. **Information Disclosure**: Hidden files (`.env`, `.git/`) blocked
4. **DoS Attacks**: 
   - CID length limits
   - Path depth limits
   - Invalid encoding rejection
5. **Invalid Requests**:
   - Malformed CIDs rejected early
   - Suspicious patterns detected
   - Null bytes blocked
6. **Method-based Attacks**:
   - Inappropriate methods rejected
   - OPTIONS properly handled for CORS

## Performance Considerations

- **Early validation**: Invalid requests rejected before resource lookup
- **Efficient checks**: Simple string operations, no regex
- **Minimal allocations**: Path normalization done once
- **Caching opportunities**: CID validation results could be cached (future enhancement)

## Future Enhancements

1. **Rate limiting per route**: Different limits for expensive vs. cheap operations
2. **CID validation caching**: Cache validation results to avoid repeated checks
3. **Request telemetry**: Track which validations are triggered most
4. **Custom error pages**: Branded error responses instead of plain text
5. **GeoIP blocking**: Block requests from specific regions if needed
6. **Request signatures**: Validate signed requests for sensitive operations

## Code Structure

```
src/gateway/
├── routing.rs          ← New production-ready routing module
│   ├── CIDValidator
│   ├── PathSanitizer
│   ├── MethodValidator
│   └── ExtensionValidator
├── server.rs           ← Updated handlers with validation
├── handler.rs          ← Unchanged (validation happens before handler)
└── mod.rs              ← Exports new validators
```

## Configuration

All validations use sensible defaults:
- CID: 30-100 characters, base32 alphabet
- Path: Maximum depth 10, alphanumeric + `/.-_`
- Methods: Route-specific whitelist
- Extensions: Comprehensive allow/block lists

## Monitoring

All validation failures are logged with:
- Timestamp
- Validation type
- Rejected value
- Reason for rejection

Example log:
```
2025-12-22T18:43:45.123Z WARN pied_piper::gateway::routing: Invalid CID: abc123 - CID too short (minimum 30 characters)
2025-12-22T18:43:46.456Z WARN pied_piper::gateway::routing: Directory traversal attempt detected: /../etc/passwd
2025-12-22T18:43:47.789Z WARN pied_piper::gateway::routing: Suspicious file extension in path: script.php
```

## Backward Compatibility

✅ All existing valid requests continue to work
✅ Web-app example still serves correctly
✅ Bundle redirect still functions
✅ WebSocket routes unaffected
✅ Health/metrics endpoints unaffected

## Conclusion

The Pied Piper gateway now has enterprise-grade routing security that:
- Validates all inputs before processing
- Provides clear error messages
- Logs security events
- Prevents common attack vectors
- Maintains high performance
- Requires zero configuration

The routing is now production-ready and can safely handle untrusted user input.
