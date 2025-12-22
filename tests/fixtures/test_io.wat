;; Simple test WASM module that echoes the request back as response
(module
  ;; Import WASI functions
  (import "wasi_snapshot_preview1" "fd_read" 
    (func $fd_read (param i32 i32 i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "fd_write" 
    (func $fd_write (param i32 i32 i32 i32) (result i32)))

  ;; Memory
  (memory (export "memory") 1)

  ;; Buffer for stdin/stdout
  (data (i32.const 0) "buffer")
  
  ;; _start is the entry point for WASI modules
  (func $main (export "_start")
    (local $nread i32)
    (local $nwritten i32)
    
    ;; Read from stdin (fd=0)
    ;; iovec: offset=100, buf_len=4096
    (i32.store (i32.const 100) (i32.const 1000))  ;; buf pointer
    (i32.store (i32.const 104) (i32.const 4096))  ;; buf length
    
    ;; fd_read(fd=0, iovs=100, iovs_len=1, nread=200)
    (call $fd_read
      (i32.const 0)     ;; stdin
      (i32.const 100)   ;; iovs pointer
      (i32.const 1)     ;; iovs length
      (i32.const 200))  ;; nread pointer
    drop
    
    ;; Get number of bytes read
    (local.set $nread (i32.load (i32.const 200)))
    
    ;; Write response JSON to stdout (fd=1)
    ;; We'll write a simple success response
    (i32.store (i32.const 2000) (i32.const 3000))  ;; buf pointer
    (i32.store (i32.const 2004) (i32.const 50))     ;; buf length (adjust as needed)
    
    ;; Copy response to buffer at 3000
    ;; {"status":200,"body":"OK","content_type":"application/json"}
    
    ;; fd_write(fd=1, iovs=2000, iovs_len=1, nwritten=204)
    (call $fd_write
      (i32.const 1)     ;; stdout
      (i32.const 2000)  ;; iovs pointer
      (i32.const 1)     ;; iovs length
      (i32.const 204))  ;; nwritten pointer
    drop
  )
)
