<script>
  let picked = $state('b');
  let options = ['a', 'b', 'c'];
</script>

<p>picked {picked}</p>
{#each options as option}
  <label><input type="radio" value={option} bind:group={picked} /> {option}</label>
{/each}
