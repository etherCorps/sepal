import { mount } from 'sepal';
import * as clipboard from 'sepal/clipboard';
import * as dialog from 'sepal/dialog';
import * as fs from 'sepal/fs';
import * as shell from 'sepal/shell';
import * as appWindow from 'sepal/window';
import './app.css';
import App from './App.svelte';

mount(App);

// Used by the host's tests, which call the APIs with arguments of their own
// (a server on a port chosen at run time, scratch directories).
declare global {
	var __api: Record<string, unknown>;
}
globalThis.__api = { clipboard, dialog, fs, shell, window: appWindow };
