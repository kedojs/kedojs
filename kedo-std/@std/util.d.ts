
declare class TypedArray {
    BYTES_PER_ELEMENT: number;
    length: number;
    byteLength: number;
    byteOffset: number;
    buffer: ArrayBuffer;
}

interface PromiseConstructor {
    withResolvers(): {
        promise: Promise<any>,
        resolve: (value: any) => void,
        reject: (reason?: any) => void,
    }
}

interface AsyncIterator<T> {
    next(value?: any): Promise<IteratorResult<T>>;
    return?(value?: any): Promise<IteratorResult<T>>;
    throw?(e?: any): Promise<IteratorResult<T>>;
}

interface AsyncIterable<T> {
    [Symbol.asyncIterator](): AsyncIterator<T>;
}
