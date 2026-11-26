<script>
  let text = $state('ab');
</script>

<input bind:value={() => text, (value) => (text = value.toUpperCase())} />
<p>{text}</p>
