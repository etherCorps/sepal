// The web platform's classes that are not ECMAScript, for both runtimes (the
// UI's, with or without the DOM layer, and the backend's): installed as
// globals when the engine has none.
//
//   URL, URLSearchParams             url.js
//   TextEncoder, TextDecoder         text.js
//   AbortController, AbortSignal     abort.js
//   fetch, Headers, Request, Response fetch.js
//   ReadableStream, WritableStream,  web-streams-polyfill (MIT)
//     TransformStream and the
//     queuing strategies
//   Blob, File, FormData             formdata-node (MIT)
//
// `Event` and `EventTarget` are not here: the DOM layer has its own, which
// know about the renderer's events.

import { Blob, File, FormData } from 'formdata-node';
import { ByteLengthQueuingStrategy, CountQueuingStrategy, ReadableStream, TransformStream, WritableStream } from 'web-streams-polyfill';
import { AbortController, AbortSignal } from './abort.js';
import { fetch, Headers, Request, Response } from './fetch.js';
import { TextDecoder, TextEncoder } from './text.js';
import { URL, URLSearchParams } from './url.js';

const classes = {
	URL,
	URLSearchParams,
	TextEncoder,
	TextDecoder,
	AbortController,
	AbortSignal,
	fetch,
	Headers,
	Request,
	Response,
	ReadableStream,
	WritableStream,
	TransformStream,
	ByteLengthQueuingStrategy,
	CountQueuingStrategy,
	Blob,
	File,
	FormData
};
for (const [name, value] of Object.entries(classes)) {
	if (typeof (/** @type {any} */ (globalThis)[name]) !== 'function') /** @type {any} */ (globalThis)[name] = value;
}

export { URL, URLSearchParams, TextEncoder, TextDecoder, AbortController, AbortSignal, fetch, Headers, Request, Response, ReadableStream, WritableStream, TransformStream, Blob, File, FormData };
