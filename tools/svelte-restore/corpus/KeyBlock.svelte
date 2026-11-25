<script>
  let version = $state(1);
</script>

{#key version}
  <em>v{version}</em>
{/key}
<button onclick={() => version++}>bump</button>
