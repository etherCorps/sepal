// packages/sepal/vite/router-entry.js for the DOM: Svelte's own `mount`, into <body>.
import './bench-emit.js'; // first: the app's `start` mark runs while the route table is imported
import { mount } from 'svelte';
import Router from '../../../packages/sepal/vite/Router.svelte';
import RouterError from '../../../packages/sepal/vite/RouterError.svelte';
import { start } from './routing.svelte.js';
import * as manifest from 'virtual:sepal/routes';

start(manifest, RouterError);
mount(Router, { target: document.body });
