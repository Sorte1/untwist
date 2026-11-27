import 'svelte/internal/disclose-version';
import 'svelte/internal/flags/legacy';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<p class="q svelte-y5yftv">t</p>`);

const $$css = {
	hash: 'svelte-y5yftv',
	code: 'p.svelte-y5yftv:where(.x) {color:teal;}'
};

export default function ZCssI1($$anchor) {
	$.append_styles($$anchor, $$css);

	var p = root();

	$.append($$anchor, p);
}