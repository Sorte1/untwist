<script>
  let { href = '/home', disabled = false } = $props();
  let title = $state('tip');
</script>

<a {href} title={title} class="link">link</a>
<button {disabled} aria-label="Action" onclick={() => (title = 'changed')}>act</button>
<img src="/a.png" alt="" />
