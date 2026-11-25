<script>
  let text = $state('hello');
</script>

<textarea bind:value={text}></textarea>
<p>{text.length}</p>
