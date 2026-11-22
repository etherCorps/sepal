import { mount } from '@ethercorps/svelte-blitz';
import * as clipboard from '@ethercorps/svelte-blitz/clipboard';
import * as dialog from '@ethercorps/svelte-blitz/dialog';
import * as fs from '@ethercorps/svelte-blitz/fs';
import * as shell from '@ethercorps/svelte-blitz/shell';
import * as appWindow from '@ethercorps/svelte-blitz/window';
import './app.css';
import App from './App.svelte';

mount(App);

// Used by the host's tests, which call the APIs with arguments of their own
// (a server on a port chosen at run time, scratch directories).
declare global {
	var __api: Record<string, unknown>;
}
globalThis.__api = { clipboard, dialog, fs, shell, window: appWindow };
