import 'svelte/internal/disclose-version';
import 'svelte/internal/flags/legacy';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<header><nav><a href="/">home</a> | <a href="/about">about</a></nav></header> <main><article><h2>Title</h2> <p>Some <strong>static</strong> text.</p></article></main>`, 1);

export default function Static($$anchor) {
	var fragment = root();

	$.next(2);
	$.append($$anchor, fragment);
}