import { myFunction } from './script.js';
import './script2.js';

// console.log("Meta Inf :", import.meta.filename, import.meta.path, import.meta.main);

const promise = Promise.resolve();

promise.then(() => {
    console.log("Here :", myFunction());

    promise.then(() => {
        console.log("Here :", new Array().test());
    })
        .catch((error) => {
            console.log(error);
        });
});

// try {
//     throw new Error(import.meta.filename);
// } catch (error) {
//     console.log(error);
// }