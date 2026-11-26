<script>
  let extra = $state({ title: 'hello', 'data-n': 1 });
</script>

<div {...extra} class="box">spread</div>
<button onclick={() => (extra = { ...extra, 'data-n': extra['data-n'] + 1 })}>bump</button>
