<script>
  let r = $state(10);
</script>

<svg width="100" height="100" viewBox="0 0 100 100">
  <circle cx="50" cy="50" {r} fill="red" />
</svg>
<button onclick={() => (r += 5)}>grow</button>
