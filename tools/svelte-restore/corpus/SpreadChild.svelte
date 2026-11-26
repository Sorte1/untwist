<script>
  import Badge from './Badge.svelte';
  let props = $state({ label: 'spread', tone: 'warn' });
</script>

<Badge {...props} />
<button class="swap" onclick={() => (props = { label: 'other' })}>swap</button>
