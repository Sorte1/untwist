<script>
  let key = $state('');
</script>

<svelte:window onkeydown={(event) => (key = event.key)} />
<p>{key}</p>
