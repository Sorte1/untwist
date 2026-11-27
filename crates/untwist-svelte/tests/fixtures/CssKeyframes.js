import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<div><p class="a b">t</p><span class="svelte-15gbs1s">s</span></div>`);

const $$css = {
	hash: 'svelte-15gbs1s',
	code: '\n@keyframes pulse { from { opacity: 0 } to { opacity: 1 } }span.svelte-15gbs1s { animation: pulse 2s;}'
};

export default function CssKeyframes($$anchor) {
	$.append_styles($$anchor, $$css);

	let on = true;
	var div = root();

	$.set_class(div, 1, 'box', null, {}, { on });
	$.append($$anchor, div);
}