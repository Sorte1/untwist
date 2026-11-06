(module
  (import "wbg" "__wbg_log_9a1b2c" (func $log (param i32 i32)))
  (import "wbg" "__wbindgen_throw" (func $throw (param i32 i32)))
  (memory (export "memory") 1)
  (func $greet (export "greet") (param i32 i32) (call $log (local.get 0) (local.get 1)))
  (func $malloc (export "__wbindgen_malloc") (param i32 i32) (result i32) (local.get 0))
)
