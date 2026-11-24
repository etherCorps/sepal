// The entry module of an app that uses the built-in router. The Vite preset
// points the build at this file; `virtual:sepal/routes` is the route table it
// generates from that directory.

import { mount } from 'sepal';
import Router from './Router.svelte';
import RouterError from './RouterError.svelte';
import { start } from './routing.svelte.js';
import * as manifest from 'virtual:sepal/routes';

start(manifest, RouterError);
mount(Router);
