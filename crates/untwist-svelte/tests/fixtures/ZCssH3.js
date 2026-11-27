import 'svelte/internal/disclose-version';
import 'svelte/internal/flags/legacy';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<span class="svelte-111sgqg">t</span>`);

const $$css = {
	hash: 'svelte-111sgqg',
	code: 'span.svelte-111sgqg:is(.z) {top:0;}'
};

export default function ZCssH3($$anchor) {
	$.append_styles($$anchor, $$css);

	var span = root();

	$.append($$anchor, span);
}