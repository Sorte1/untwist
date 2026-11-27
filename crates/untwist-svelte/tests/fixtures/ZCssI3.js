import 'svelte/internal/disclose-version';
import 'svelte/internal/flags/legacy';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<p class="q svelte-112fckh">t</p>`);

const $$css = {
	hash: 'svelte-112fckh',
	code: '#app.q.svelte-112fckh {color:teal;}'
};

export default function ZCssI3($$anchor) {
	$.append_styles($$anchor, $$css);

	var p = root();

	$.append($$anchor, p);
}