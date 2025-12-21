# Phase 3 - Content Distribution (In Progress)

## Overview

Phase 3 focuses on distributing WebAssembly modules across the decentralized network using libp2p's DHT and request-response protocols.

## Progress

### ✅ Completed

1. **Module Structure Created**
   - Created `src/content/` directory with modular architecture
   - Defined protocol types: `ModuleRequest` and `ModuleResponse`
   - Implemented search and metadata structures

2. **Protocol Components Implemented**
   - `protocol.rs`: Request/response message types and protocol definition
   - `publisher.rs`: Module metadata creation and announcement logic
   - `discovery.rs`: DHT query tracking and module discovery
   - `provider.rs`: Module serving and request handling

3. **Network Integration**
   - Added request-response behavior to `PiedPiperBehaviour`
   - Integrated content protocol into `NetworkNode`
   - Added event handling for module requests/responses

4. **Dependencies Added**
   - `libp2p` with `request-response`, `json`, `cbor` features
   - `serde_json` for serialization
   - `futures` for async I/O
   - `async-trait` for trait implementations

### 🔄 In Progress

**Codec Implementation** - Currently fixing the ContentCodec implementation
- Issue: libp2p's `Codec` trait has specific lifetime bounds that need to match exactly
- The trait expects specific async patterns without `async_trait` macro
- Need to implement without async_trait or match the trait's expected signature

### ⏳ Pending

1. **Module Publishing**
   - Wire up DHT record creation
   - Implement `publish_module()` in NetworkNode
   - Add GossipSub announcement broadcasting

2. **Module Discovery**
   - Implement DHT lookups by CID and name
   - Add provider search functionality
   - Integrate with ModuleLoader

3. **Module Fetching**
   - Implement request-response client logic
   - Add integrity verification (CID matching)
   - Handle failed requests and retries

4. **CLI Integration**
   - Update `deploy` command to publish to network
   - Update `run` command to fetch from network if not cached
   - Add `search` command for module discovery

5. **Testing**
   - Multi-node module distribution tests
   - Stress testing with large modules
   - Network partition resilience

## Architecture

```
┌───────────────────────────────────────────────────┐
│              Content Distribution                  │
├───────────────────────────────────────────────────┤
│                                                    │
│  ┌──────────────┐    ┌───────────────┐           │
│  │  Publisher   │───▶│   DHT Record  │           │
│  │              │    │   + GossipSub │           │
│  └──────────────┘    └───────────────┘           │
│                                                    │
│  ┌──────────────┐    ┌───────────────┐           │
│  │  Discovery   │◀───│  DHT Queries  │           │
│  │              │    │  Name→CID     │           │
│  └──────────────┘    └───────────────┘           │
│                                                    │
│  ┌──────────────┐    ┌───────────────┐           │
│  │  Provider    │◀──▶│ Req/Response  │           │
│  │              │    │    Protocol   │           │
│  └──────────────┘    └───────────────┘           │
│                                                    │
└───────────────────────────────────────────────────┘
                       │
                       ▼
            ┌──────────────────┐
            │  LibP2P Network  │
            │  • DHT           │
            │  • GossipSub     │
            │  • Req/Resp      │
            └──────────────────┘
```

## Protocol Design

### Request Types
- `GetModule`: Fetch module bytes by CID
- `GetModuleInfo`: Fetch module metadata
- `SearchByName`: Find modules by name
- `ListModules`: Get all modules from a peer

### Response Types
- `Module`: Module bytes + CID
- `ModuleInfo`: Metadata (name, version, size, deps, author)
- `SearchResults`: List of matching modules
- `ModuleList`: CIDs of all available modules
- `NotFound`: Module doesn't exist
- `Error`: Error message

### DHT Records
- **Module Metadata**: `module:{CID}` → JSON metadata
- **Name Mapping**: `name:{name}:{version}` → CID

### GossipSub Topics
- `pied-piper/modules/announcements`: New module announcements

## Next Steps

1. **Fix Codec Implementation**
   - Remove `async_trait` usage or match exact trait signature
   - Consider using libp2p's built-in JSON codec if available
   - Test request/response flow

2. **Complete Publisher Integration**
   - Add `publish_module()` method to NetworkNode
   - Implement DHT `put_record()` calls
   - Add GossipSub `publish()` for announcements

3. **Implement Discovery Methods**
   - Add `find_module()` method (by CID)
   - Add `search_modules()` method (by name)
   - Handle DHT query results

4. **Wire Up CLI Commands**
   - Implement `deploy` command logic
   - Update `run` command to fetch remotely
   - Add progress indicators

5. **Testing**
   - Create integration tests
   - Test with multiple nodes
   - Verify CID integrity

## Files Created

- `src/content/mod.rs` - Module exports
- `src/content/protocol.rs` - Protocol types and codec
- `src/content/publisher.rs` - Module publishing logic
- `src/content/discovery.rs` - Module discovery and DHT queries
- `src/content/provider.rs` - Module serving and request handling

## Compilation Status

**Current**: Does not compile due to Codec trait implementation issues

**Errors to Fix**:
1. Codec trait lifetime bounds mismatch
2. Need to remove or properly use async_trait
3. Type annotations needed in provider.rs (HashMap types)

## Estimated Time to Complete

- Codec implementation: 1-2 hours
- Publisher integration: 2-3 hours
- Discovery implementation: 2-3 hours
- CLI integration: 2-3 hours
- Testing: 3-4 hours

**Total**: ~10-15 hours of development time

---

**Status**: Phase 3 initialization complete, codec implementation in progress  
**Next Session**: Fix ContentCodec, then continue with publisher integration
