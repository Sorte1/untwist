<script>
  let { items = [] } = $props();
  let list = $state(items);

  function drop() {
    list = list.slice(1);
  }
</script>

<ul>
  {#each list as item, i (item.id)}
    <li data-index={i}>{item.name}</li>
  {:else}
    <li>empty</li>
  {/each}
</ul>
<button onclick={drop}>drop</button>
