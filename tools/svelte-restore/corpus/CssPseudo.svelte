<script>let on = $state(true);</script>
<div class="box" class:on><p class="a b">t</p><span>s</span></div>
<style>
p:is(.a, .b):where(.a) > *, div:global(.g):not(span) { top: 0 } :root { --v: 1 } :global(*) { box-sizing: border-box } .box:global(.a.b) > span { top: 1px } p:global(:hover) { left: 0 }
</style>
