<script>
  let on = $state(false);
</script>

<label><input type="checkbox" bind:checked={on} /> toggle</label>
{#if on}
  <p>on</p>
{/if}
