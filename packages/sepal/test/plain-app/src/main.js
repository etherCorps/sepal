// An app with no `static/` directory of its own (test/icon.test.js).
import { mount } from 'sepal';
import List from './List.svelte';

mount(List, { props: { ids: ['plain entry'] } });
