import 'svelte/internal/disclose-version';
import 'svelte/internal/flags/legacy';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<div class="b svelte-a4q790"><p class="q svelte-a4q790">t</p><span class="svelte-a4q790">s</span></div>`);

const $$css = {
	hash: 'svelte-a4q790',
	code: '.dyn.b.svelte-a4q790 {color:teal;}.q.svelte-a4q790:hover.x {left:0;}p.svelte-a4q790.x::before {content:\'a\';}span.svelte-a4q790[data-k="v.w"] {top:0;}span.svelte-a4q790#id {top:1px;}'
};

export default function ZCssGlobalFirst($$anchor) {
	$.append_styles($$anchor, $$css);

	var div = root();

	$.append($$anchor, div);
}