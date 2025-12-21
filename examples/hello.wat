(module
  ;; A simple WebAssembly module that adds two numbers
  (func $add (export "add") (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.add
  )
  
  ;; A function that returns a constant
  (func $get_answer (export "get_answer") (result i32)
    i32.const 42
  )
  
  ;; A function that multiplies two numbers
  (func $multiply (export "multiply") (param $x i32) (param $y i32) (result i32)
    local.get $x
    local.get $y
    i32.mul
  )
)
