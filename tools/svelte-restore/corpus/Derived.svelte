<script>
  let n = $state(1);
  let a = $derived(n * 2);
  let b = $derived.by(() => { let total = 0; for (let i = 0; i < n; i++) total += i; return total; });
  let c = $derived.by(() => n + 1);
  let d = $derived((() => n + 2)());
  let { x, y } = $derived({ x: n, y: n * 3 });
</script>
<button onclick={() => n++}>{a} {b} {c} {d} {x} {y}</button>
