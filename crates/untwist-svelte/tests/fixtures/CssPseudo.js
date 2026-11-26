import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<div><p class="a b svelte-dr9bb">t</p><span class="svelte-dr9bb">s</span></div>`);

const $$css = {
	hash: 'svelte-dr9bb',
	code: ' div.svelte-dr9bb.g:not(span) {top:0}:root {--v: 1}* {box-sizing:border-box}.box.svelte-dr9bb.a.b > span:where(.svelte-dr9bb) {top:1px}p.svelte-dr9bb:hover {left:0}'
};

export default function CssPseudo($$anchor) {
	$.append_styles($$anchor, $$css);

	let on = true;
	var div = root();

	$.set_class(div, 1, 'box svelte-dr9bb', null, {}, { on });
	$.append($$anchor, div);
}