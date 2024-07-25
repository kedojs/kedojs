const isObject = (value: any) => {
    return value !== null && typeof value === 'object';
}

const isPrototypeOf = (value: any, prototype: any) => {
    return Object.getPrototypeOf(value) === prototype;
}

const assert = (condition: boolean, message?: string) => {
    if (!condition) {
        throw new Error(message || 'Illegal state');
    }
}

const isTypedArray = (value: any): value is TypedArray => {
    return ArrayBuffer.isView(value) && !(value instanceof DataView);
}

const getTag = (value: any) => {
    return Object.prototype.toString.call(value);
}

const isArrayBuffer = (value: any): value is ArrayBuffer => {
    return isObject(value) && getTag(value) === '[object ArrayBuffer]';
}

// https://tc39.es/ecma262/multipage/abstract-operations.html#sec-getiteratorfrommethod
const getIteratorFromMethod = (object: any, method: any) => {
    // 1. Let iterator be ? Call(method, obj).
    const iterator = method.call(object);
    // 2. If iterator is not an Object, throw a TypeError exception.
    if (!isObject(iterator)) throw new TypeError('Iterator must be an object');
    // 3. Let nextMethod be ? Get(iterator, "next").
    const nextMethod = iterator.next;
    // 4. Let iteratorRecord be the Iterator Record { [[Iterator]]: iterator, [[NextMethod]]: nextMethod, [[Done]]: false }.
    const iteratorRecord = { iterator, nextMethod, done: false };
    // 5. Return iteratorRecord.
    return iteratorRecord;
}


// https://tc39.es/ecma262/multipage/abstract-operations.html#sec-getiterator
const getIterator = (object: any): Iterator<any> | AsyncIterator<any> => {
    if (object[Symbol.asyncIterator]) {
        const method = object[Symbol.asyncIterator];
        const iterator = method.call(object);
        if (!isObject(iterator)) throw new TypeError('Iterator must be an object');
        if (!iterator.next || typeof iterator.next !== 'function') 
            throw new TypeError('Iterator must have a next method');
        return iterator;
    }

    if (object[Symbol.iterator]) {
        const method = object[Symbol.iterator];
        const iterator = method.call(object);
        if (!isObject(iterator)) throw new TypeError('Iterator must be an object');
        if (!iterator.next || typeof iterator.next !== 'function') 
            throw new TypeError('Iterator must have a next method');
        return iterator;
    }

    throw new TypeError('Value is not iterable');
}

const AsyncGeneratorPrototype = Object.getPrototypeOf(async function* () {});
const AsyncIteratorPrototype = Object.getPrototypeOf(AsyncGeneratorPrototype);

class Deferred<T = void> {
    promise: Promise<T>;
    resolve: (value: any) => void;
    reject: (reason?: any) => void;

    constructor() {
        const { promise, resolve, reject } = Promise.withResolvers();
        this.promise = promise;
        this.resolve = resolve;
        this.reject = reject;
    }
}

export { 
    isObject, 
    Deferred, 
    assert, 
    isPrototypeOf, 
    isTypedArray, 
    getTag, 
    isArrayBuffer, 
    getIterator,
    AsyncGeneratorPrototype,
    AsyncIteratorPrototype,
 };
