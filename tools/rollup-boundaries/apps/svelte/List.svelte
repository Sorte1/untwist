<script>
  import Item from "./Item.svelte";
  let { items, title } = $props();
  let remaining = $derived(items.filter((item) => !item.done).length);
</script>

<h2>{title} ({remaining} left)</h2>
<ul>
  {#each items as item (item.id)}
    <Item label={item.label} done={item.done} />
  {/each}
</ul>
