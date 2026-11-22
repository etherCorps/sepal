// Used when routing is switched off.
import { mount } from '@ethercorps/svelte-blitz';
import List from './List.svelte';

mount(List, { props: { ids: ['plain entry'], has_fallback: false } });
