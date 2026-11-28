// `FormData` as a request or response body, both ways: `multipart/form-data`
// (with files) and `application/x-www-form-urlencoded`, as browsers write and
// read them. The classes are formdata-node's (`web.js`).

import { File, FormData } from 'formdata-node';
import { TextDecoder, TextEncoder } from './text.js';
import { URLSearchParams } from './url.js';

const encoder = new TextEncoder();
const decoder = new TextDecoder();

/** A boundary that will not be in the content, as browsers choose them. */
export function make_boundary() {
	let random = '';
	for (let i = 0; i < 24; i++) random += Math.floor(Math.random() * 16).toString(16);
	return `----SepalFormBoundary${random}`;
}

/** A name or file name inside a quoted header parameter, as HTML escapes it. @param {string} text */
const quote = (text) => text.replace(/"/g, '%22').replace(/\r/g, '%0D').replace(/\n/g, '%0A');

/**
 * `form` as a `multipart/form-data` body with `boundary`.
 * @param {FormData} form
 * @param {string} boundary
 * @returns {Promise<Uint8Array>}
 */
export async function encode_multipart(form, boundary) {
	/** @type {Uint8Array[]} */
	const parts = [];
	for (const [name, value] of form) {
		let head = `--${boundary}\r\nContent-Disposition: form-data; name="${quote(name)}"`;
		if (typeof value === 'string') {
			parts.push(encoder.encode(`${head}\r\n\r\n`), encoder.encode(value.replace(/\r?\n|\r/g, '\r\n')), encoder.encode('\r\n'));
		} else {
			head += `; filename="${quote(value.name)}"\r\nContent-Type: ${value.type || 'application/octet-stream'}\r\n\r\n`;
			parts.push(encoder.encode(head), new Uint8Array(await value.arrayBuffer()), encoder.encode('\r\n'));
		}
	}
	parts.push(encoder.encode(`--${boundary}--\r\n`));
	return concat(parts);
}

/** @param {Uint8Array[]} parts */
export function concat(parts) {
	const all = new Uint8Array(parts.reduce((total, part) => total + part.length, 0));
	let at = 0;
	for (const part of parts) {
		all.set(part, at);
		at += part.length;
	}
	return all;
}

/**
 * A body as `FormData`, by its content type, as `formData()` reads it.
 * @param {Uint8Array} bytes
 * @param {string | null} content_type
 * @returns {FormData}
 */
export function parse_form(bytes, content_type) {
	const type = (content_type ?? '').toLowerCase();
	if (type.startsWith('application/x-www-form-urlencoded')) {
		const form = new FormData();
		for (const [name, value] of new URLSearchParams(decoder.decode(bytes))) form.append(name, value);
		return form;
	}
	if (type.startsWith('multipart/form-data')) {
		const boundary = /boundary=(?:"([^"]+)"|([^;\s]+))/i.exec(content_type ?? '');
		if (boundary) return parse_multipart(bytes, boundary[1] ?? boundary[2]);
	}
	throw new TypeError(`formData(): the body is ${content_type ?? 'of no type'}, not a form`);
}

/** @param {Uint8Array} bytes @param {string} boundary */
function parse_multipart(bytes, boundary) {
	const form = new FormData();
	const delimiter = encoder.encode(`--${boundary}`);
	let at = find(bytes, delimiter, 0);
	if (at < 0) throw new TypeError('formData(): the multipart body has no boundary');
	for (;;) {
		at += delimiter.length;
		// `--` after a delimiter ends the body.
		if (bytes[at] === 45 && bytes[at + 1] === 45) break;
		at = skip_line_end(bytes, at);
		const head_end = find(bytes, encoder.encode('\r\n\r\n'), at);
		if (head_end < 0) throw new TypeError('formData(): a part of the multipart body has no end of headers');
		const headers = decoder.decode(bytes.subarray(at, head_end));
		const start = head_end + 4;
		const next = find(bytes, delimiter, start);
		if (next < 0) throw new TypeError('formData(): the multipart body is cut off');
		// The part's content ends with the line break before the next delimiter.
		let end = next;
		if (bytes[end - 1] === 10) end--;
		if (bytes[end - 1] === 13) end--;
		add_part(form, headers, bytes.slice(start, end));
		at = next;
	}
	return form;
}

/** @param {Uint8Array} bytes @param {number} at */
function skip_line_end(bytes, at) {
	if (bytes[at] === 13) at++;
	if (bytes[at] === 10) at++;
	return at;
}

/** @param {FormData} form @param {string} headers @param {Uint8Array} content */
function add_part(form, headers, content) {
	/** @type {Record<string, string>} */
	const fields = {};
	for (const line of headers.split(/\r?\n/)) {
		const colon = line.indexOf(':');
		if (colon > 0) fields[line.slice(0, colon).trim().toLowerCase()] = line.slice(colon + 1).trim();
	}
	const disposition = fields['content-disposition'] ?? '';
	const name = parameter(disposition, 'name');
	if (name === null) return;
	const filename = parameter(disposition, 'filename');
	if (filename === null) form.append(name, decoder.decode(content));
	else form.append(name, new File([content], filename, { type: fields['content-type'] ?? 'application/octet-stream' }));
}

/** A quoted or bare parameter of a header value. @param {string} header @param {string} name */
function parameter(header, name) {
	const match = new RegExp(`(?:^|;)\\s*${name}=(?:"((?:[^"\\\\]|\\\\.)*)"|([^;]*))`, 'i').exec(header);
	if (!match) return null;
	const value = match[1] ?? match[2].trim();
	return value.replace(/%22/g, '"').replace(/%0D/gi, '\r').replace(/%0A/gi, '\n');
}

/** Where `needle` starts in `bytes` from `from`, or -1. @param {Uint8Array} bytes @param {Uint8Array} needle @param {number} from */
function find(bytes, needle, from) {
	const first = needle[0];
	outer: for (let i = bytes.indexOf(first, from); i >= 0 && i <= bytes.length - needle.length; i = bytes.indexOf(first, i + 1)) {
		for (let j = 1; j < needle.length; j++) if (bytes[i + j] !== needle[j]) continue outer;
		return i;
	}
	return -1;
}
