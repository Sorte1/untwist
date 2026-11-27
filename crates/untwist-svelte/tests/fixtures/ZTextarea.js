import 'svelte/internal/disclose-version';
import 'svelte/internal/flags/legacy';
import * as $ from 'svelte/internal/client';

var root = $.from_html(
	`<textarea>

x</textarea> <textarea>
y</textarea> <textarea>  z </textarea>`,
	1
);

export default function ZTextarea($$anchor) {
	var fragment = root();

	$.next(4);
	$.append($$anchor, fragment);
}