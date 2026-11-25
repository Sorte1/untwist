<script>
  let count = $state(0);
  let doubled = $derived(count * 2);

  function increment() {
    count += 1;
  }
</script>

<button onclick={increment}>clicks: {count}</button>
<p>doubled is {doubled}</p>
