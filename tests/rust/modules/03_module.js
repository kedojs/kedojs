import { add, div } from './math.js';
import { sub, mul } from './math.js';
import { mean } from './stats.js';
import { median } from './stats.js';

globalThis.addOne = (a) => add(a, 1);
globalThis.divTwo = (a) => div(a, 2);
globalThis.subOne = (a) => sub(a, 1);
globalThis.mulTwo = (a) => mul(a, 2);

globalThis.mean = mean;
globalThis.median = median;