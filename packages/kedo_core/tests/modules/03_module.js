import { add, div, mul, sub } from './math.js';
import { mean, median } from './stats.js';

globalThis.addOne = (a) => add(a, 1);
globalThis.divTwo = (a) => div(a, 2);
globalThis.subOne = (a) => sub(a, 1);
globalThis.mulTwo = (a) => mul(a, 2);

globalThis.mean = mean;
globalThis.median = median;