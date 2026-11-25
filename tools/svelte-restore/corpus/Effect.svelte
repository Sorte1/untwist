<script>
  let count = $state(0);

  $effect(() => {
    console.log('count is', count);
  });
</script>

<button onclick={() => count++}>{count}</button>
