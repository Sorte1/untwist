import 'svelte/internal/disclose-version';
import 'svelte/internal/flags/legacy';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<div class="b svelte-xvh1o5">t</div>`);

const $$css = {
	hash: 'svelte-xvh1o5',
	code: '.dyn.b.svelte-xvh1o5 {color:teal;}'
};

export default function ZCssG1($$anchor) {
	$.append_styles($$anchor, $$css);

	var div = root();

	$.append($$anchor, div);
}