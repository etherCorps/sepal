// packages/vite/src/router-entry.js for the DOM: Svelte's own `mount`, into <body>.
import './bench-emit.js'; // first: the app's `start` mark runs while the route table is imported
import { mount } from 'svelte';
import Router from '../../../packages/vite/src/Router.svelte';
import RouterError from '../../../packages/vite/src/RouterError.svelte';
import { start } from './routing.svelte.js';
import * as manifest from 'virtual:svelte-blitz/routes';

start(manifest, RouterError);
mount(Router, { target: document.body });
