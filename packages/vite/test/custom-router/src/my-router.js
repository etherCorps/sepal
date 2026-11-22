// A router of the app's own: it is the entry module, and it gets the route
// table the preset generated. This one shows a list of the routes it was given.
import { mount } from '@ethercorps/svelte-blitz';
import { fallback, routes } from 'virtual:svelte-blitz/routes';
import List from './List.svelte';

mount(List, { props: { ids: routes.map((route) => route.id), has_fallback: fallback.error !== null } });
