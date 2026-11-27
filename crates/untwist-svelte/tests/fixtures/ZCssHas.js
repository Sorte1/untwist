import 'svelte/internal/disclose-version';
import 'svelte/internal/flags/legacy';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<div class="b svelte-h7n4c9"><p class="q svelte-h7n4c9">t</p><span class="svelte-h7n4c9">s</span></div>`);

const $$css = {
	hash: 'svelte-h7n4c9',
	code: 'p.svelte-h7n4c9:has(.x) {color:teal;}div.svelte-h7n4c9:not(.y) {left:0;}span.svelte-h7n4c9:is(.z) {top:0;}.outer:is(.z) span.svelte-h7n4c9 {top:2px;}'
};

export default function ZCssHas($$anchor) {
	$.append_styles($$anchor, $$css);

	var div = root();

	$.append($$anchor, div);
}