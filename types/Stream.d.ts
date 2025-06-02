type ReadableStreamState = "readable" | "closed" | "errored";

interface ReadRequest<T = any> {
    chunkSteps: (chunk: T) => void;
    closeSteps: () => void;
    errorSteps: (e: any) => void;
}

/**
 * Represents a request for reading data into a destination with callbacks for different stages.
 *
 * @interface ReadIntoRequest
 *
 * @property {function} chunkSteps - Callback function that processes each chunk of data
 * as it is read. Takes an ArrayBufferView parameter containing the chunk data.
 *
 * @property {function} closeSteps - Callback function that executes when the reading operation
 * completes. Optionally receives a final ArrayBufferView chunk.
 *
 * @property {function} errorSteps - Callback function that handles any errors that occur
 * during the reading operation. Receives the error as a parameter.
 */
interface ReadIntoRequest {
    chunkSteps: (chunk: ArrayBufferView) => void;
    closeSteps: (chunk?: ArrayBufferView) => void;
    errorSteps: (error: any) => void;
}

interface PullIntoDescriptor {
    buffer: ArrayBuffer;
    bufferByteLength: number;
    byteOffset: number;
    byteLength: number;
    bytesFilled: number;
    minimumFill: number;
    elementSize: number;
    viewConstructor: any;
    readerType: "default" | "byob" | "none";
}

interface QueueingStrategyInit {
    highWaterMark?: number;
}

interface QueuingStrategySizeCallback<T = any> {
    (chunk: T): number;
}

interface QueuingStrategy<T = any> {
    size?: QueuingStrategySizeCallback<T>;
    highWaterMark?: number;
}

interface ArrayBuffer {
    transfer(newByteLength?: number): ArrayBuffer;
}

interface ReadableStreamGetReaderOptions {
    mode?: ReadableStreamReaderMode;
}

interface ReadableStreamIteratorOptions {
    preventCancel?: boolean;
}

interface UnderlyingSourceStartCallback<R = any> {
    (controller: ReadableStreamDefaultController): R;
}

interface UnderlyingSourcePullCallback<R = any> {
    (controller: ReadableStreamDefaultController): Promise<R>;
}

interface UnderlyingSourceCancelCallback<R = any> {
    (reason: any): Promise<R>;
}

interface UnderlyingSource<R = any> {
    start?: UnderlyingSourceStartCallback<R>;
    pull?: UnderlyingSourcePullCallback<R>;
    cancel?: UnderlyingSourceCancelCallback;
    autoAllocateChunkSize?: number;
    type?: ReadableStreamType;
}

interface StreamPipeOptions {
    preventClose?: boolean;
    preventAbort?: boolean;
    preventCancel?: boolean;
}

interface ValueWithSize<T = any> {
    value: T;
    size: number;
}

interface ReadableStreamBYOBReaderReadOptions {
    min?: number;
}

declare interface ReadableStreamGenericReader {
    /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/ReadableStreamBYOBReader/closed) */
    readonly closed: Promise<undefined>;
    /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/ReadableStreamBYOBReader/cancel) */
    cancel(reason: any): Promise<void>;
}

interface IReadableStreamDefaultReader extends ReadableStreamGenericReader {
    read(): Promise<ReadableStreamReadResult<any>>;
    releaseLock(): void;
}

interface IReadableStreamBYOBReader extends ReadableStreamGenericReader {
    read(
        view: ArrayBufferView,
        options?: ReadableStreamBYOBReaderReadOptions,
    ): Promise<ReadableStreamReadResult<any>>;
    releaseLock(): void;
}

declare interface ReadableStreamReadResult<T> {
    value: T;
    done: boolean;
}

declare type ReadableStreamReaderMode = "byob";

declare type ReadableStreamType = "bytes";

declare module "@kedo/stream" {
    export {
        ByteLengthQueuingStrategy,
        CountQueuingStrategy,
        ReadableByteStreamController,
        ReadableStream,
        ReadableStreamBYOBReader,
        ReadableStreamBYOBRequest,
        ReadableStreamDefaultController,
        ReadableStreamDefaultReader,
    } from "@kedo:int/std/stream";
}

declare module "@kedo:int/std/stream" {
    export enum StreamError {
        Closed = -1.0,
        ChannelFull = -2.0,
        ReceiverTaken = -3.0,
        SendError = -4.0,
        Empty = -5.0,
    }
    /**
     * Represents a readable stream of data that can be consumed via readers or async iteration.
     *
     * @remarks
     * - `locked`: Indicates whether the stream is currently locked to a reader.
     * - `cancel(reason)`: Cancels the stream, signaling that the consumer no longer needs its data.
     * - `from(iterable)`: Creates a new readable stream from an iterable or async iterable.
     * - `getReader(options)`: Returns a reader (default or BYOB) to read from the stream.
     * - `values(args)`: Returns an async iterable iterator to read chunks from the stream.
     * - `[Symbol.asyncIterator]()`: Allows the stream to be used in `for await...of` loops.
     *
     * @public
     */
    export class ReadableStream {
        constructor(
            underlyingSource?: UnderlyingSource | null,
            strategy?: QueuingStrategy,
        );
        get locked(): boolean;
        cancel(reason: any): Promise<void>;
        static from<T>(
            asyncIterable: Iterable<T> | AsyncIterable<T>,
        ): ReadableStream;
        getReader<
            T = ReadableStreamDefaultReader | ReadableStreamBYOBReader,
        >(options?: { mode: "byob" }): T;
        values(args?: { preventCancel?: boolean }): AsyncIterableIterator<any>;
        [Symbol.asyncIterator]<T>(): AsyncIterableIterator<T>;
    }

    export class ReadableStreamBYOBRequest {
        get view(): ArrayBufferView<ArrayBuffer> | ReadableByteStreamController;
        respond(bytesWritten: number): void;
        respondWithNewView(view: ArrayBufferView): void;
    }

    export class ReadableByteStreamController {
        get byobRequest(): any;
        get desiredSize(): number;
        close(): void;
        enqueue(chunk: ArrayBufferView): void;
        error(e: any): void;
    }

    export class ReadableStreamBYOBReader implements IReadableStreamBYOBReader {
        constructor(stream: ReadableStream);
        read(
            view: ArrayBufferView,
            options?: ReadableStreamBYOBReaderReadOptions | undefined,
        ): Promise<ReadableStreamReadResult<any>>;
        cancel(reason: any): Promise<void>;
        get closed(): any;
        releaseLock(): void;
    }

    export class ReadableStreamDefaultReader
        implements IReadableStreamDefaultReader
    {
        constructor(stream: ReadableStream);
        get closed(): any;
        cancel(reason: any): Promise<void>;
        read(): any;
        releaseLock(): void;
    }

    export class ReadableStreamDefaultController {
        get desiredSize(): number;
        close(): void;
        error(e: any): void;
        enqueue(chunk: any): void;
    }

    // https://streams.spec.whatwg.org/#bytelengthqueuingstrategy
    // A common queuing strategy when dealing with bytes is to wait until
    // the accumulated byteLength properties of the incoming chunks reaches a specified high-water mark.
    // As such, this is provided as a built-in queuing strategy that can be used when constructing streams.
    export class ByteLengthQueuingStrategy implements QueuingStrategy {
        constructor(init: QueueingStrategyInit);
        size: QueuingStrategySizeCallback;
        get highWaterMark(): number;
    }

    export class CountQueuingStrategy implements QueuingStrategy {
        constructor(init: QueueingStrategyInit);
        size: QueuingStrategySizeCallback;
        get highWaterMark(): number;
    }

    export function isDisturbed(stream: ReadableStream): boolean;
    export function isInReadableState(stream: ReadableStream): boolean;
    export function isErrored(stream: ReadableStream): boolean;
    export function readableStreamEnqueue(
        stream: ReadableStream,
        chunk: ArrayBufferView,
    ): void;
    export function readableStreamCloseByteController(
        stream: ReadableStream,
    ): void;
    export function readableStreamResource(
        stream: ReadableStream,
        size?: number,
    ): import("@kedo:op/web").ReadableStreamResource;
    export function readableStreamClose(stream: ReadableStream): void;
    export const createReadableStream: <T>(
        startAlgorithm: () => void,
        pullAlgorithm: () => Promise<void>,
        cancelAlgorithm: (reason: any) => Promise<void>,
        highWaterMark?: number,
        sizeAlgorithm?: QueuingStrategySizeCallback<T>,
    ) => ReadableStream;
    export const createReadableByteStream: (
        startAlgorithm: () => void,
        pullAlgorithm: () => Promise<void>,
        cancelAlgorithm: (reason: any) => Promise<void>,
        highWaterMark?: number,
    ) => ReadableStream;
}
