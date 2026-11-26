<script>
  import { fade, fly } from 'svelte/transition';
  let show = $state(true);
</script>

{#if show}
  <p transition:fade={{ duration: 0 }}>both</p>
  <p in:fly={{ y: 10, duration: 0 }} out:fade|global={{ duration: 0 }}>split</p>
{/if}
<button onclick={() => (show = !show)}>toggle</button>
