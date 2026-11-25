<script>
  const LIMIT = 3;
  let { start = 0 } = $props();
  let value = $state(start);
  let capped = $derived(Math.min(value, LIMIT));

  function step(by) {
    value = value + by;
  }
</script>

<button onclick={() => step(1)}>up</button>
<button onclick={() => step(-1)}>down</button>
<output>{capped} of {LIMIT}</output>
