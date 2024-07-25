
const headers = new Headers({
    "Content-Type": 'application/json',
    "X-Custom-Header": 'custom value'
});

console.log("Is Instanceof: ", headers instanceof Headers); // true

console.log(headers.get('Content-Type')); // application/json
console.log(headers.get('X-Custom-Header')); // custom value

headers.set('Content-Type', 'text/plain');
console.log(headers.get('Content-Type')); // text/plain

const headers2 = new Headers({
    "Content-Type": 'html/text',
    "X-Custom-Header": 'Token'
});

console.log(headers2.get('Content-Type')); // html/text
console.log(headers2.get('X-Custom-Header')); // Token

console.log("Is Equal: ", headers.get('Content-Type') === headers2.get('Content-Type')); // false

console.log(headers.get('Content-Type')); // text/plain
console.log(headers2.has('Content-Type')); // true
console.log(headers.has('Content-Type')); // true
console.log(headers.has('None')); // false

headers2.delete('Content-Type');
console.log(headers2.has('Content-Type')); // false

const keys = headers.keys();
for (let key of keys) {
    console.log(key);
}

const keys2 = headers.keys();
for (let key of keys2) {
    console.log(key);
}

const values = headers.values();
for (let value of values) {
    console.log(value);
}

const entries = headers.entries();
for (let [key, value] of headers) {
    console.log(key, value);
}

for (let index = 0; index < 5000; index++) {
    let header_tmp = new Headers({
        "Content-Type": 'application/json',
        "X-Custom-Header": 'custom value'
    });

    for (let header of header_tmp) {
        // console.log(header);
    }

    // console.log(header_tmp.get('Content-Type')); // application/json
}
