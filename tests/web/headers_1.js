// Create a new Headers object
let headers = new Headers();
console.log('Initial Headers:', [...headers]);

// Test appending headers
headers.append('Content-Type', 'application/json');
headers.append('X-Custom-Header', 'CustomValue');
console.log('After append:', [...headers]);

// Test getting headers
console.log('Get Content-Type:', headers.get('Content-Type'));
console.log('Get X-Custom-Header:', headers.get('X-Custom-Header'));
console.log('Get non-existent header:', headers.get('Non-Existent'));

// Test setting headers
headers.set('Content-Type', 'text/plain');
console.log('After set Content-Type:', [...headers]);

// Test deleting headers
headers.delete('X-Custom-Header');
console.log('After delete X-Custom-Header:', [...headers]);

// Test has method
console.log('Has Content-Type:', headers.has('Content-Type'));
console.log('Has X-Custom-Header:', headers.has('X-Custom-Header'));
console.log('Has non-existent header:', headers.has('Non-Existent'));

// Test forEach method
headers.forEach((value, name) => {
    console.log(`Header ${name}: ${value}`);
});

// Test getSetCookie
headers.append('Set-Cookie', 'sessionId=abc123');
console.log('Get Set-Cookie:', headers.getSetCookie());

// Test iterator
for (let [name, value] of headers) {
    console.log(`Iterator - Header ${name}: ${value}`);
}

// Test entries, keys, and values methods
console.log('Entries:', [...headers.entries()]);
console.log('Keys:', [...headers.keys()]);
console.log('Values:', [...headers.values()]);

// Test immutability (attempting to change an immutable headers object)
// try {
//     headers[_headersGuard] = 'immutable';
//     headers.set('Content-Type', 'application/xml');
// } catch (e) {
//     console.error('Error setting header on immutable object:', e.message);
// }

// Reset immutability for further testing
// headers[_headersGuard] = 'none';

// Test invalid header values
try {
    headers.append('Invalid-Header', 'Invalid Value\x00');
} catch (e) {
    console.error('Error appending invalid header value:', e.message);
}

// Test invalid header names
try {
    headers.append('Invalid-Header-Ñame!', 'ValidValue');
} catch (e) {
    console.error('Error appending invalid header name:', e.message);
}

// Test setting Set-Cookie header
headers.set('Set-Cookie', 'sessionId=xyz789');
console.log('Set-Cookie after set:', headers.getSetCookie());

console.log('Final Headers:', [...headers]);
