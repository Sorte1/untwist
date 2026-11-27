<script>
  let n = $state(1);
  let d = $derived(n * 2);
  let list = $derived([n, n + 1]);
  let fn = $derived(() => n);
  function over() { d = 100; }
</script>
<button class="a" onclick={() => n++}>n</button>
<button class="b" onclick={over}>o</button>
<p>{d} {list.join()} {fn()}</p>
