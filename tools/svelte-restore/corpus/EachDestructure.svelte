<script>
  let rows = $state([{ id: 1, name: 'a' }, { id: 2, name: 'b' }]);
</script>

{#each rows as { id, name } (id)}
  <p>{id}: {name}</p>
{/each}
<button onclick={() => rows.reverse()}>flip</button>
