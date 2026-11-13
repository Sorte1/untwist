<script>
  import List from "./List.svelte";
  let { name } = $props();
  let items = $state([
    { id: 1, label: "parse", done: true },
    { id: 2, label: "split", done: false },
    { id: 3, label: "print", done: false },
  ]);
</script>

<main>
  <h1>Hello {name}</h1>
  <List {items} title="Todo" />
  {#if items.length > 2}<p>busy</p>{:else}<p>idle</p>{/if}
</main>
