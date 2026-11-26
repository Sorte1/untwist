<script>
  let rows = $state([{ id: 1, pos: { x: 3 } }, { id: 2, pos: {} }]);
</script>

{#each rows as { id, pos: { x = 9 } }, i (id)}
  <p>{i}: {id} at {x}</p>
{/each}
{#each [[1, 2], [3]] as [a, b = 0]}
  <span>{a}-{b}</span>
{/each}
<button onclick={() => rows.reverse()}>flip</button>
