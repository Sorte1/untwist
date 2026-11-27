import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<div><p class="a b">t</p><span>s</span></div>`);

const $$css = {
	hash: 'svelte-ayg3st',
	code: '.box.svelte-ayg3st {.q {right:1px;}p {left:0}}'
};

export default function CssGlobalBlock($$anchor) {
	$.append_styles($$anchor, $$css);

	let on = true;
	var div = root();

	$.set_class(div, 1, 'box svelte-ayg3st', null, {}, { on });
	$.append($$anchor, div);
}