import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<p></p> <pre>  keep   spaces  </pre>`, 1);

export default function Escapes($$anchor) {
	let a = 1;
	var fragment = root();
	var p = $.first_child(fragment);

	p.textContent = 'braces {1} & <tag> done';
	$.next(2);
	$.append($$anchor, fragment);
}