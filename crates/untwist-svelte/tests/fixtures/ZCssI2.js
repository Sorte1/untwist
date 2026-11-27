import 'svelte/internal/disclose-version';
import 'svelte/internal/flags/legacy';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<p class="q svelte-10mh86y">t</p>`);

const $$css = {
	hash: 'svelte-10mh86y',
	code: 'p.svelte-10mh86y:has(> .x) {color:teal;}'
};

export default function ZCssI2($$anchor) {
	$.append_styles($$anchor, $$css);

	var p = root();

	$.append($$anchor, p);
}