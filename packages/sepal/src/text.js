// `TextEncoder` and `TextDecoder` for an engine without them (QuickJS).
// UTF-8 only, which is all `TextEncoder` ever does and what `TextDecoder`
// does unless asked otherwise.

export class TextEncoder {
	encoding = 'utf-8';

	/** @param {string} [text] */
	encode(text = '') {
		text = String(text);
		const bytes = [];
		for (let i = 0; i < text.length; i++) {
			let code = /** @type {number} */ (text.codePointAt(i));
			if (code > 0xffff) i++;
			// A surrogate with no partner cannot be encoded: it becomes U+FFFD.
			else if (code >= 0xd800 && code <= 0xdfff) code = 0xfffd;

			if (code < 0x80) bytes.push(code);
			else if (code < 0x800) bytes.push(0xc0 | (code >> 6), 0x80 | (code & 0x3f));
			else if (code < 0x10000) bytes.push(0xe0 | (code >> 12), 0x80 | ((code >> 6) & 0x3f), 0x80 | (code & 0x3f));
			else bytes.push(0xf0 | (code >> 18), 0x80 | ((code >> 12) & 0x3f), 0x80 | ((code >> 6) & 0x3f), 0x80 | (code & 0x3f));
		}
		return new Uint8Array(bytes);
	}
}

export class TextDecoder {
	/** @param {string} [label] @param {{ fatal?: boolean, ignoreBOM?: boolean }} [options] */
	constructor(label = 'utf-8', options = {}) {
		if (!/^(utf-?8|unicode-1-1-utf-8)$/i.test(label)) throw new RangeError(`The "${label}" encoding is not supported`);
		this.encoding = 'utf-8';
		this.fatal = options.fatal ?? false;
		this.ignoreBOM = options.ignoreBOM ?? false;
		/** Bytes of a character that the last chunk ended in the middle of. @type {number[]} */
		this.pending = [];
	}

	/** @param {ArrayBuffer | ArrayBufferView} [input] @param {{ stream?: boolean }} [options] */
	decode(input, options = {}) {
		const view =
			input === undefined ? new Uint8Array(0) : input instanceof ArrayBuffer ? new Uint8Array(input) : new Uint8Array(input.buffer, input.byteOffset, input.byteLength);
		const bytes = this.pending.length > 0 ? [...this.pending, ...view] : view;
		this.pending = [];
		let out = '';
		const invalid = () => {
			if (this.fatal) throw new TypeError('The encoded data was not valid');
			out += '�';
		};

		let i = !this.ignoreBOM && bytes[0] === 0xef && bytes[1] === 0xbb && bytes[2] === 0xbf ? 3 : 0;
		while (i < bytes.length) {
			const first = bytes[i];
			if (first < 0x80) {
				out += String.fromCharCode(first);
				i++;
				continue;
			}
			const length = first >= 0xf0 && first <= 0xf4 ? 4 : first >= 0xe0 ? 3 : first >= 0xc2 && first < 0xe0 ? 2 : 0;
			if (length === 0) {
				invalid();
				i++;
				continue;
			}
			if (i + length > bytes.length) {
				// Cut off. In a stream the rest may still come.
				const rest = Array.from(bytes.slice(i));
				if (rest.slice(1).every((byte) => (byte & 0xc0) === 0x80)) {
					if (options.stream) this.pending = rest;
					else invalid();
					break;
				}
			}
			let code = first & (length === 2 ? 0x1f : length === 3 ? 0x0f : 0x07);
			let read = 1;
			while (read < length && (bytes[i + read] & 0xc0) === 0x80) code = (code << 6) | (bytes[i + read++] & 0x3f);
			const smallest = length === 2 ? 0x80 : length === 3 ? 0x800 : 0x10000;
			if (read < length || code < smallest || code > 0x10ffff || (code >= 0xd800 && code <= 0xdfff)) {
				invalid();
				i += Math.max(1, read < length ? read : 1);
				continue;
			}
			out += String.fromCodePoint(code);
			i += length;
		}
		return out;
	}
}
