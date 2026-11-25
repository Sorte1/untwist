<script>
  let items = $state.raw([1, 2]);
  let pushed = $state.raw([1]);
</script>

<button class="replace" onclick={() => (items = [...items, items.length + 1])}>{items.length}</button>
<button class="push" onclick={() => pushed.push(2)}>{pushed.length}</button>
