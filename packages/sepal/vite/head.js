// Svelte refuses `<svelte:head>` when it compiles for a custom renderer: there
// is no <head> to put anything in. Apps written for the web use it on nearly
// every page, mostly for one thing, the title. So this preprocessor takes the
// block out and keeps that:
//
//   <svelte:head><title>{item.name} - Shop</title></svelte:head>
//
// becomes an expression that sets `document.title` whenever what it reads
// changes. With the DOM layer loaded that is the page's <title>, which the
// window shows; without it the expression does nothing. Anything else
// in the block (<meta>, <link>, <script>) means nothing in an app and is
// dropped, with a warning at build time.
//
// Runs in Node.

import { parse } from 'svelte/compiler';

/**
 * @param {any} node
 * @param {(head: any) => void} visit
 */
function heads(node, visit) {
	if (node === null || typeof node !== 'object') return;
	if (Array.isArray(node)) {
		for (const child of node) heads(child, visit);
		return;
	}
	if (node.type === 'SvelteHead') return visit(node);
	for (const key of Object.keys(node)) {
		if (key !== 'metadata' && key !== 'parent') heads(node[key], visit);
	}
}

/**
 * The title as a template literal: text as it is, `{expressions}` as `${…}`.
 * @param {any} title a TitleElement
 * @param {string} source
 */
function title_literal(title, source) {
	let out = '';
	for (const part of title.fragment.nodes) {
		if (part.type === 'Text') out += part.data.replace(/[\\`$]/g, '\\$&');
		else if (part.type === 'ExpressionTag') out += '${' + source.slice(part.expression.start, part.expression.end) + '}';
	}
	return '`' + out.replace(/\s+/g, ' ').trim() + '`';
}

/** @returns {import('svelte/compiler').PreprocessorGroup} */
export function head() {
	return {
		name: 'sepal:head',
		markup({ content, filename }) {
			if (!content.includes('<svelte:head')) return;
			/** @type {any[]} */
			const found = [];
			heads(parse(content, { modern: true, filename }).fragment, (node) => found.push(node));
			if (found.length === 0) return;

			let code = content;
			// Back to front, so earlier positions stay valid.
			for (const node of found.reverse()) {
				const title = node.fragment.nodes.find((/** @type {any} */ child) => child.type === 'TitleElement');
				const others = node.fragment.nodes.filter((/** @type {any} */ child) => child !== title && !(child.type === 'Text' && child.data.trim() === '') && child.type !== 'Comment');
				if (others.length > 0) {
					const names = [...new Set(others.map((/** @type {any} */ child) => (child.name ? `<${child.name}>` : 'text')))].join(', ');
					console.warn(`sepal: ${names} in <svelte:head> is ignored: an app has no <head> (${filename ?? 'unknown file'})`);
				}
				// An expression in the template: Svelte runs it again when what the
				// title reads changes, in runes mode and in legacy mode alike. It
				// renders as an empty string.
				const replacement = title ? `{(globalThis.document && (globalThis.document.title = ${title_literal(title, content)}), '')}` : '';
				// As many lines as before, so error messages below still point right.
				const lines = '\n'.repeat(content.slice(node.start, node.end).split('\n').length - 1);
				code = code.slice(0, node.start) + replacement + lines + code.slice(node.end);
			}
			return { code };
		}
	};
}
