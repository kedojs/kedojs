declare const isObject: (value: any) => boolean;
declare const isPrototypeOf: (value: any, prototype: any) => boolean;
declare const assert: (condition: boolean, message?: string) => void;
declare const isTypedArray: (value: any) => value is TypedArray;
declare const isDataView: (value: any) => value is DataView;
declare const getTag: (value: any) => any;
declare const isArrayBuffer: (value: any) => value is ArrayBuffer;
declare const getIterator: (object: any) => Iterator<any> | AsyncIterator<any>;
declare const AsyncGeneratorPrototype: any;
declare const AsyncIteratorPrototype: any;
declare class Deferred<T = void> {
    promise: Promise<T>;
    resolve: (value: any) => void;
    reject: (reason?: any) => void;
    constructor();
}
export { isObject, Deferred, assert, isPrototypeOf, isTypedArray, isDataView, getTag, isArrayBuffer, getIterator, AsyncGeneratorPrototype, AsyncIteratorPrototype, };
