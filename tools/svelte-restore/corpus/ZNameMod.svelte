<script module>
  export function untrack(f) { console.log('mine-untrack'); return f() + 1; }
  export function tick() { console.log('mine-tick'); return 7; }
  export function get(o) { return o.v * 2; }
  export function set(o, v) { o.v = v + 100; }
</script>
<script>
  let n = $state(1);
  const o = { v: 1 };
  function go() { set(o, n); n = untrack(() => n) + tick() + get(o); }
</script>
<button onclick={go}>{n}</button>
