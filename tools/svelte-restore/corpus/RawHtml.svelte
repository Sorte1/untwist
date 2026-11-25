<script>
  let markup = $state('<b>bold</b>');
</script>

<div>{@html markup}</div>
<button onclick={() => (markup = '<i>it</i>')}>swap</button>
