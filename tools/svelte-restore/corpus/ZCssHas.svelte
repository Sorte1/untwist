<div class="b"><p class="q">t</p><span>s</span></div>
<style>
p:has(:global(.x)) { color: teal; }
div:not(:global(.y)) { left: 0; }
span:is(:global(.z), .w) { top: 0; }
:global(.outer):is(:global(.z)) span { top: 2px; }
</style>
