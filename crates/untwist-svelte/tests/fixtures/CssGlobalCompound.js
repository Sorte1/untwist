import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<div><p class="a b svelte-1162247">t</p><span class="svelte-1162247">s</span></div>`);

const $$css = {
	hash: 'svelte-1162247',
	code: 'span.svelte-1162247.dyn {color:teal;}.out p.a.svelte-1162247 {top:0;}p.svelte-1162247.x:hover {left:0}'
};

export default function CssGlobalCompound($$anchor) {
	$.append_styles($$anchor, $$css);

	let on = true;
	var div = root();

	$.set_class(div, 1, 'box', null, {}, { on });
	$.append($$anchor, div);
}