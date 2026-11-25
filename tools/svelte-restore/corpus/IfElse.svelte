<script>
  let n = $state(0);
</script>

<button onclick={() => n++}>next</button>
{#if n === 0}
  <p>zero</p>
{:else if n === 1}
  <p>one</p>
{:else}
  <p>many: {n}</p>
{/if}
