<script>
  import { flip } from 'svelte/animate';
  let items = $state([1, 2, 3]);
</script>

<ul>
  {#each items as item (item)}
    <li animate:flip={{ duration: 0 }}>{item}</li>
  {/each}
</ul>
<button onclick={() => items.reverse()}>flip</button>
