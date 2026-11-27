import 'svelte/internal/disclose-version';
import 'svelte/internal/flags/legacy';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<p class="q svelte-zm6bfc">t</p>`);

const $$css = {
	hash: 'svelte-zm6bfc',
	code: '[data-theme].q.svelte-zm6bfc {color:teal;}p.q.svelte-zm6bfc:hover {top:0;}'
};

export default function ZCssI4($$anchor) {
	$.append_styles($$anchor, $$css);

	var p = root();

	$.append($$anchor, p);
}