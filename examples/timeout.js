// import { sum } from './examples/functions.js';

// console.log('Sum: ', sum(1, 2));

const id = setTimeout(() => {
    console.log('Hello, world! Timeout: 1');
}, 1000);

console.log('Timeout 1 id: ', id);
clearTimeout(id);

let i = 0;

const interval = setInterval((name) => {
    i += 1;
    console.log('Hello, world! From Interval: ', i, name);
}, 3000, "Kevin");

const id2 = setTimeout((name, age) => {
    console.log('Hello, world! Timeout: 6: ', name, age);
    clearInterval(interval);

    // Kedo.readFile('./examples/data.txt').then((data) => {
    //     console.log(data);
    //     setTimeout(() => {
    //         console.log('KedoJS 8');
    //         setTimeout(() => {
    //             console.log('KedoJS 9');
    //             setTimeout(() => {
    //                 console.log('KedoJS 10');
    //             }, 1000);
    //         }, 1000);
    //     }, 1000);
    // });

    console.log('Hello, world! setTimeout Id Interval: ', interval);

    setTimeout(() => {
        console.log('KedoJS 7');
    }, 1000);

}, 9000, "KEvin", 20);

Promise.resolve().then(() => {
    setTimeout(() => {
        console.log('KedoJS 3');
        setTimeout(() => {
            console.log('KedoJS 4');
            setTimeout(() => {
                console.log('KedoJS 11');
            }, 12000);
        }, 1000);

        setTimeout(() => {
            console.log('KedoJS ME 2');
            setTimeout(() => {
                console.log('KedoJS ME 1');
            }, 10000);
        }, 1500);
    }, 1000);
});

Promise.resolve().then(() => {
    setTimeout(() => {
        console.log('KedoJS 3');
    }, 1000);
});

Promise.resolve().then(() => {
    setTimeout(() => {
        console.log('KedoJS 3');
        // Kedo.readFile('./examples/data.txt').then((data) => {
        //     console.log(data);
        //     setTimeout(() => {
        //         console.log('KedoJS 8');
        //         setTimeout(() => {
        //             console.log('KedoJS 9');
        //             setTimeout(() => {
        //                 console.log('KedoJS 10');
        //             }, 1000);
        //         }, 1000);
        //     }, 1000);
        // });
    }, 1000);
});

console.log('Timeout 2 id: ', id2);