<script>
  let { title = 'List', items = ['x', 'y'] } = $props();
  let filter = $state('');
  let shown = $derived(items.filter((item) => item.includes(filter)));
</script>

<section>
  <h2>{title} ({shown.length})</h2>
  <input bind:value={filter} placeholder="filter" />
  {#if shown.length === 0}
    <p class="empty">nothing matches "{filter}"</p>
  {:else}
    <ul>
      {#each shown as item, index (item)}
        <li>{index + 1}. {item}</li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  section > h2 { font-size: 1.2em; }
  .empty { color: gray; }
  ul li:first-child { font-weight: bold; }
</style>
