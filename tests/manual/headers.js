
let headers_array = [];

(() => {
console.log('IIFE');

const headers = new Headers({
    "Content-Type": 'application/json',
    "X-Custom-Header": 'custom value'
});

console.log(headers.get('Content-Type')); // application/json
console.log(headers.get('X-Custom-Header')); // custom value

headers.set('Content-Type', 'text/plain');
console.log(headers.get('Content-Type')); // text/plain

setTimeout(() => {
    console.log(headers.get('Content-Type')); // application/json
}, 1000, "Timeout 1");

headers.delete('Content-Type');
console.log(headers instanceof Headers); // true

let counter2 = 0;
let timeout_2 = setInterval((arg) => {
    counter2++;
    console.log(arg);

    const mockFunc = () => {
        for (let index = 0; index < 10000; index++) {
            let header_tmp = new Headers([
                ['Content-Type', 'application/json'],
                ['X-Custom-Header', 'custom value ${index}']
            ]);

            for (let [name, value] of header_tmp) {
                // console.log(header_tmp.get(name));
                // header_tmp.get(name);
            }

            headers_array.push(header_tmp);
        }
    }

    mockFunc();
}, 500, "Timeout 2");

console.log("Timeout 2 ID: ", timeout_2);

let counter = 0;
let timeout_3 = setInterval((arg) => {
    console.log(arg);
    counter++;

    if (counter === 20) {
        console.log('Clearing interval');
        clearInterval(timeout_2);
        const size = headers_array.length;
        console.log('Header List Size: ', size);
        for (let index = 0; index < size; index++) {
            // console.log(`Removing Item: ${index} of ${size}`);
            headers_array.pop();
        }
    }

}, 1000, "Timeout 3");
console.log("Timeout 3 ID: ", timeout_3);

setTimeout((arg) => {
    // clearInterval(interval);
    console.log(arg);
    clearInterval(timeout_3);
    headers_array = [];
    let size = headers_array.length;
    console.log('clearing interval: ', headers_array.length);
    for (let index = 0; index < size; index++) {
        // console.log(`Removing Item: ${index} of ${size}`);
        headers_array.pop();
    }
}, 25000, "Timeout 4");
})();


setInterval(() => {
    const other = []
    console.log('Interval');
    // for (let index = 0; index < 10000; index++) {
    //     // let header_tmp = new Headers({
    //     //     "Content-Type": 'application/json',
    //     //     "X-Custom-Header": 'custom value'
    //     // });

    //     let header_tmp = new Headers([
    //         ['Content-Type', 'application/json'],
    //         ['X-Custom-Header', 'custom value']
    //     ]);

    //     // console.log(header_tmp.get('Content-Type')); // application/json
    //     // console.log(header_tmp.get('X-Custom-Header')); // custom value
    //     // console.log(header_tmp instanceof Headers); // true
    //     // console.log(header_tmp.has('Content-Type')); // true

    //     other.push(header_tmp);
    // }
}, 10000, "Interval");
