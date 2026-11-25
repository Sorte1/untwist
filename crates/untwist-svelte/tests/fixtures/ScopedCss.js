import 'svelte/internal/disclose-version';
import 'svelte/internal/flags/legacy';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<p class="note svelte-15vhjat">styled</p> <span class="svelte-15vhjat">plain</span>`, 1);

const $$css = {
	hash: 'svelte-15vhjat',
	code: '.note.svelte-15vhjat {color:red;font-weight:bold;}span.svelte-15vhjat {margin:0 4px;}'
};

export default function ScopedCss($$anchor) {
	$.append_styles($$anchor, $$css);

	var fragment = root();

	$.next(2);
	$.append($$anchor, fragment);
}