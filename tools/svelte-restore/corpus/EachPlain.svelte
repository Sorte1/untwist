<script>
  let words = $state(['alpha', 'beta']);
</script>

{#each words as word}
  <span>{word} </span>
{/each}
<button onclick={() => words.push('gamma')}>add</button>
