(module
  (import "./env.js" "log" (func $log (param i32)))
  (func $add (export "add") (param i32 i32) (result i32) (i32.add (local.get 0) (local.get 1)))
)
