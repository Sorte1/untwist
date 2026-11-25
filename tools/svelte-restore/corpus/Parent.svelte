<script>
  import Badge from './Badge.svelte';
  let picked = $state('none');
  let count = $state(1);
</script>

<Badge label="first" onpick={(value) => (picked = value)}>inner {count}</Badge>
<Badge label="second" tone="warn" />
<p>picked {picked}</p>
<button class="more" onclick={() => count++}>more</button>
