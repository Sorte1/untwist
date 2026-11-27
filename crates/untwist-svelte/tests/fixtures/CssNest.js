import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<div><p class="a b svelte-19rao6z">t</p><span>s</span></div>`);

const $$css = {
	hash: 'svelte-19rao6z',
	code: '.box.svelte-19rao6z {& p:where(.svelte-19rao6z) {color:green;}.a:where(.svelte-19rao6z) {color:pink;}&:hover {top:0}}'
};

export default function CssNest($$anchor) {
	$.append_styles($$anchor, $$css);

	let on = true;
	var div = root();

	$.set_class(div, 1, 'box svelte-19rao6z', null, {}, { on });
	$.append($$anchor, div);
}