<script>
  let values = $state([1, 2]);
</script>

{#each values as value}
  {@const double = value * 2}
  <i>{double}</i>
{/each}
