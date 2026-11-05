(module
  (import "a" "a" (func $emscripten_memcpy_big (param i32 i32 i32)))
  (import "a" "b" (func $abort))
  (memory (export "c") 1)
  (func $main (export "d") (result i32) (i32.const 0))
  (func $malloc (export "e") (param i32) (result i32) (local.get 0))
)
