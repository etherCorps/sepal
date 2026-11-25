// What a build may put in an app's page: the checks both presets run on
// index.html and the stylesheets it loads.
//
// The host takes what Vite and SvelteKit write: a head with a title, meta,
// stylesheets (<link> and <style>) and modulepreload; markup in the body;
// inline classic scripts and module scripts. Scripts, stylesheets,
// modulepreload targets and fonts come from the app folder only, never from
// the network, and the host refuses them too. Anything else a browser would
// do with a page is left out, which the build says.
//
// Runs in Node.

/** A URL that goes to the network: `http:`, `https:`, or `//host`. */
const REMOTE = /^\s*(?:https?:)?\/\//i;

/** @param {string} tag @returns {Record<string, string>} */
function attributes(tag) {
	/** @type {Record<string, string>} */
	const found = {};
	for (const [, name, double, single, bare] of tag.matchAll(/\s([^\s"'=<>/]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'=<>`]+)))?/g)) {
		found[name.toLowerCase()] = double ?? single ?? bare ?? '';
	}
	return found;
}

const FONTS = 'Fonts load only from the app folder: self-host them, for example with Fontsource (https://fontsource.org).';

/**
 * What in a stylesheet would load from the network: `@import` of a remote
 * stylesheet, and fonts in `@font-face`. Images are not checked: they are
 * refused when they load, as today.
 * @param {string} css
 * @param {string} where the file, for the message
 * @returns {string[]}
 */
export function check_css(css, where) {
	const errors = [];
	const code = css.replace(/\/\*[\s\S]*?\*\//g, '');
	for (const [, url] of code.matchAll(/@import\s+(?:url\(\s*)?["']?([^"')\s;]+)/gi)) {
		if (REMOTE.test(url)) errors.push(`${where} imports the stylesheet ${url}. Stylesheets load only from the app folder: import it from a package, or put it in static/. ${FONTS}`);
	}
	for (const [block] of code.matchAll(/@font-face\s*\{[^}]*\}/gi)) {
		for (const [, url] of block.matchAll(/url\(\s*["']?([^"')\s]+)/gi)) {
			if (REMOTE.test(url)) errors.push(`${where} loads the font ${url}. ${FONTS}`);
		}
	}
	return errors;
}

/**
 * Check a page. `errors` stop the build: something would load from the
 * network. `warnings` are what the host leaves out.
 * @param {string} html
 * @param {string} where the file, for the messages
 * @returns {{ errors: string[], warnings: string[] }}
 */
export function check_page(html, where = 'index.html') {
	/** @type {string[]} */
	const errors = [];
	/** @type {Set<string>} */
	const warnings = new Set();
	const page = html.replace(/<!--[\s\S]*?-->/g, '');

	for (const [tag, name] of page.matchAll(/<([a-z][a-z0-9-]*)\b[^>]*>/gi)) {
		const attrs = attributes(tag.slice(name.length + 1, -1));
		const element = name.toLowerCase();
		if (Object.keys(attrs).some((attr) => /^on[a-z]/.test(attr))) warnings.add('event handler attributes (onclick and the like) do nothing: nothing runs them');
		if (element === 'iframe') warnings.add('<iframe> is not supported: it stays empty');
		if (element === 'script') {
			const type = (attrs.type ?? '').trim().toLowerCase();
			if (attrs.src !== undefined && REMOTE.test(attrs.src)) {
				errors.push(`${where} loads the script ${attrs.src}. Scripts load only from the app folder: install the package and import it, or put the file in static/.`);
			} else if (type === 'importmap') {
				warnings.add('import maps are not supported: imports are resolved by the build');
			} else if (attrs.src !== undefined && type !== 'module' && /^(|text\/javascript|application\/javascript)$/.test(type)) {
				warnings.add('<script src> without type="module" is not run: load scripts as modules');
			}
		}
		if (element === 'link' && attrs.href !== undefined && REMOTE.test(attrs.href)) {
			const rel = (attrs.rel ?? '').toLowerCase().split(/\s+/);
			if (rel.includes('stylesheet')) {
				errors.push(`${where} loads the stylesheet ${attrs.href}. Stylesheets load only from the app folder: import it from a package, or put it in static/. ${FONTS}`);
			} else if (rel.includes('modulepreload')) {
				errors.push(`${where} preloads the module ${attrs.href}. Scripts load only from the app folder: install the package and import it.`);
			} else if (rel.includes('preload') && (attrs.as ?? '').toLowerCase() === 'font') {
				errors.push(`${where} preloads the font ${attrs.href}. ${FONTS}`);
			}
		}
	}
	for (const [, css] of page.matchAll(/<style\b[^>]*>([\s\S]*?)<\/style>/gi)) errors.push(...check_css(css, `a <style> in ${where}`));
	for (const [, code] of page.matchAll(/<script\b[^>]*>([\s\S]*?)<\/script>/gi)) {
		if (/\bdocument\.write(ln)?\s*\(/.test(code)) warnings.add('document.write() does nothing');
	}
	return { errors, warnings: [...warnings].map((warning) => `${where}: ${warning}`) };
}
