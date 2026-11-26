import { value } from './lib/dep.js';

globalThis.order.push('main ' + value + ' ' + import.meta.url);
