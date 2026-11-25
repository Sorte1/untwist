<script>
  let { value = $bindable(0) } = $props();
</script>

<button onclick={() => value++}>{value}</button>
