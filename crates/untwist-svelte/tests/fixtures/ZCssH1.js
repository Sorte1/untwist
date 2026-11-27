import 'svelte/internal/disclose-version';
import 'svelte/internal/flags/legacy';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<p class="q svelte-y5c1p6">t</p>`);

const $$css = {
	hash: 'svelte-y5c1p6',
	code: 'p.svelte-y5c1p6:has(.x) {color:teal;}'
};

export default function ZCssH1($$anchor) {
	$.append_styles($$anchor, $$css);

	var p = root();

	$.append($$anchor, p);
}