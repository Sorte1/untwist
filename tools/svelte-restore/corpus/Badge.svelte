<script>
  let { label, tone = 'info', children, onpick } = $props();
</script>

<span class="badge {tone}">
  {label}
  {@render children?.()}
  <button onclick={() => onpick?.(label)}>pick</button>
</span>
