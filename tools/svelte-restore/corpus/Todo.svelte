<script>
  let todos = $state([{ id: 1, text: 'write', done: false }]);
  let draft = $state('');
  let nextId = 2;
  let remaining = $derived(todos.filter((todo) => !todo.done).length);

  function add() {
    if (!draft) return;
    todos.push({ id: nextId++, text: draft, done: false });
    draft = '';
  }

  function remove(id) {
    todos = todos.filter((todo) => todo.id !== id);
  }
</script>

<input bind:value={draft} placeholder="new todo" />
<button class="add" onclick={add}>add</button>
<ul>
  {#each todos as todo (todo.id)}
    <li class:done={todo.done}>
      <input type="checkbox" bind:checked={todo.done} />
      {todo.text}
      <button class="remove" onclick={() => remove(todo.id)}>x</button>
    </li>
  {/each}
</ul>
<p>{remaining} left</p>

<style>
  .done {
    text-decoration: line-through;
  }
</style>
