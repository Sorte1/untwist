<script>
  let count = $state(0);
  $effect.pre(() => {
    console.log('before', count);
  });
  $effect(() => {
    console.log('after', count);
    return () => console.log('cleanup', count);
  });
</script>

<button onclick={() => (count += 2)}>{count}</button>
