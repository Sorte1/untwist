<script>
  let log = $state('');

  function onInput(event) {
    log = 'input ' + event.target.value;
  }
</script>

<input oninput={onInput} />
<button onclick={() => (log = 'clicked')} onmouseenter={() => (log = 'hover')}>go</button>
<p>{log}</p>
