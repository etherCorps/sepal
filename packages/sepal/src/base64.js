// Bytes cross the host boundary as base64. The engine has no `atob`/`btoa`.

const ALPHABET = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/';
/** @type {Record<string, number>} */
const VALUES = {};
for (let index = 0; index < ALPHABET.length; index++) VALUES[ALPHABET[index]] = index;

/** @param {Uint8Array} bytes */
export function encode(bytes) {
	let text = '';
	for (let at = 0; at < bytes.length; at += 3) {
		const [a, b, c] = [bytes[at], bytes[at + 1], bytes[at + 2]];
		text += ALPHABET[a >> 2] + ALPHABET[((a & 3) << 4) | ((b ?? 0) >> 4)];
		text += b === undefined ? '=' : ALPHABET[((b & 15) << 2) | ((c ?? 0) >> 6)];
		text += c === undefined ? '=' : ALPHABET[c & 63];
	}
	return text;
}

/** @param {string} text */
export function decode(text) {
	const padding = text.endsWith('==') ? 2 : text.endsWith('=') ? 1 : 0;
	const bytes = new Uint8Array((text.length / 4) * 3 - padding);
	for (let at = 0, out = 0; at < text.length; at += 4) {
		const group =
			(VALUES[text[at]] << 18) | (VALUES[text[at + 1]] << 12) | ((VALUES[text[at + 2]] ?? 0) << 6) | (VALUES[text[at + 3]] ?? 0);
		if (out < bytes.length) bytes[out++] = group >> 16;
		if (out < bytes.length) bytes[out++] = (group >> 8) & 255;
		if (out < bytes.length) bytes[out++] = group & 255;
	}
	return bytes;
}

/**
 * Whatever byte container the caller has, as a `Uint8Array`.
 * @param {ArrayBuffer | ArrayBufferView} data
 */
export function to_bytes(data) {
	if (data instanceof Uint8Array) return data;
	if (ArrayBuffer.isView(data)) return new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
	return new Uint8Array(data);
}
