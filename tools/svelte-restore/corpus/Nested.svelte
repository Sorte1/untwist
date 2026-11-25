<script>
  let groups = $state([{ name: 'g1', items: ['a', 'b'] }, { name: 'g2', items: [] }]);
  let open = $state(true);
</script>

<button onclick={() => (open = !open)}>toggle</button>
{#if open}
  {#each groups as group}
    <section>
      <h3>{group.name}</h3>
      {#if group.items.length}
        <ol>
          {#each group.items as item}
            <li>{item}</li>
          {/each}
        </ol>
      {:else}
        <p>none</p>
      {/if}
    </section>
  {/each}
{/if}
