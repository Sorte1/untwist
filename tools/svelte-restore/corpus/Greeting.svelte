<script>
  let { name = 'world', excited = false } = $props();
</script>

<h1>Hello {name}{excited ? '!' : '.'}</h1>
