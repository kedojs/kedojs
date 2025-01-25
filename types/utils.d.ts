declare module "@kedo/utils" {
    const isObject: (value: any) => boolean;
    const isPrototypeOf: (value: any, prototype: any) => boolean;
    const assert: (condition: boolean, message?: string) => void;
    const isTypedArray: (value: any) => value is TypedArray;
    const isDataView: (value: any) => value is DataView;
    const getTag: (value: any) => any;
    const isArrayBuffer: (value: any) => value is ArrayBuffer;
    const getIterator: (object: any) => Iterator<any> | AsyncIterator<any>;
    const AsyncGeneratorPrototype: any;
    const AsyncIteratorPrototype: any;
    type CallbackFunction = (...args: any[]) => void;
    function promisify<T>(fn: CallbackFunction): (this: any, ...args: any[]) => Promise<T>;
    function asyncOp<T, Args extends any[]>(fn: AsyncFunctionCallback<T, Args>, ...args: Args): Promise<T>;

    class Deferred<T = void> {
        promise: Promise<T>;
        resolve: (value: any) => void;
        reject: (reason?: any) => void;
        constructor();
    }

    export { assert, AsyncGeneratorPrototype, AsyncIteratorPrototype, asyncOp, Deferred, getIterator, getTag, isArrayBuffer, isDataView, isObject, isPrototypeOf, isTypedArray, promisify };
}

type OpStyleCallback<T> = (error: Error | null | undefined, result: T) => void;

type AsyncFunctionCallback<T, Args extends any[]> = (
    ...args: [...Args, OpStyleCallback<T>]
) => void;

interface AsyncIterator<T> {
    next(value?: any): Promise<IteratorResult<T>>;
    return?(value?: any): Promise<IteratorResult<T>>;
    throw?(e?: any): Promise<IteratorResult<T>>;
}

interface AsyncIterable<T> {
    [Symbol.asyncIterator](): AsyncIterator<T>;
}

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