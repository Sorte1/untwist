<script>
  let { label, done = false } = $props();
</script>

<li class:done>{done ? "x" : " "} {label}</li>
