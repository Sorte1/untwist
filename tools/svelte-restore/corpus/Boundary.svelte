<script>
  let broken = $state(false);
  function explode() {
    if (broken) throw new Error('boom');
    return 'fine';
  }
</script>

<svelte:boundary>
  <p>{explode()}</p>
  {#snippet failed(error, reset)}
    <p class="err">{error.message}</p>
  {/snippet}
</svelte:boundary>
<button onclick={() => (broken = true)}>break</button>
