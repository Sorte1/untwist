<script module>
  export function untrack(f) { console.log('mine-untrack'); return f() + 1; }
  export function tick() { console.log('mine-tick'); return 7; }
</script>
<script>
  let n = $state(1);
  function go() { n = untrack(() => n) + tick(); }
</script>
<button onclick={go}>{n}</button>
