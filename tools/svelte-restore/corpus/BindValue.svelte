<script>
  let text = $state('start');
</script>

<input bind:value={text} />
<p>{text.toUpperCase()}</p>
