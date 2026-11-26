// Svelte before anything of Sepal's: its runtime loads first here, and
// keeps its own copy of `Object.getOwnPropertyDescriptor` as it does.
// sepal() puts Sepal's runtime, and its engine fixes, first in the page's
// modules anyway (crates/host/tests/bind_rest.rs). Components need no help:
// Svelte's output for a custom renderer imports the renderer first.
import { flushSync } from 'svelte';
import App from './App.svelte';
import { mount } from 'sepal';

mount(App);
flushSync();
