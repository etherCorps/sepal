// Our TextEncoder and TextDecoder against Node's.
import assert from 'node:assert/strict';
import { TextEncoder as Encoder, TextDecoder as Decoder } from '../src/text.js';

const samples = ['', 'plain ascii', 'é ü ñ', '日本語のテキスト', '🙂 emoji 👩‍👩‍👧 and 𝒳', 'mixed: a é 日 🙂\n\t', '\ud800 lone', 'ends lone \udfff', '\0 nul'];
for (const text of samples) {
	const theirs = new TextEncoder().encode(text);
	assert.deepEqual([...new Encoder().encode(text)], [...theirs], `encode ${JSON.stringify(text)}`);
	assert.equal(new Decoder().decode(theirs), new TextDecoder().decode(theirs), `decode ${JSON.stringify(text)}`);
	// In pieces, cut at every position.
	for (let cut = 0; cut <= theirs.length; cut++) {
		const decoder = new Decoder();
		const whole = decoder.decode(theirs.slice(0, cut), { stream: true }) + decoder.decode(theirs.slice(cut));
		assert.equal(whole, new TextDecoder().decode(theirs), `decode ${JSON.stringify(text)} cut at ${cut}`);
	}
}
for (const bytes of [[0xff], [0xc3], [0xe2, 0x82], [0xc0, 0x80], [0xed, 0xa0, 0x80], [0x61, 0x80, 0x62], [0xf4, 0x90, 0x80, 0x80], [0xef, 0xbb, 0xbf, 0x61]]) {
	const view = new Uint8Array(bytes);
	assert.equal(new Decoder().decode(view), new TextDecoder().decode(view), `decode bytes ${bytes}`);
}
assert.throws(() => new Decoder('utf-8', { fatal: true }).decode(new Uint8Array([0xff])), TypeError);
assert.throws(() => new Decoder('latin1'), RangeError);
console.log('text: ok');
