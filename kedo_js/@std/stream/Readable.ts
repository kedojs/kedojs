import { Queue } from "@kedo/ds";
import {
    assert,
    AsyncIteratorPrototype,
    Deferred,
    getIterator,
    isArrayBuffer,
    isObject,
    isPrototypeOf,
    isTypedArray,
} from "@kedo/utils";
import {
    is_array_buffer_detached,
    op_close_unbounded_stream,
    op_write_sync_unbounded_stream,
    op_write_unbounded_stream,
    UnboundedReadableStreamResource,
} from "@kedo:op/web";

interface ReadableStreamGenericReader {
    [_closedPromise]: Deferred;
    [_stream]?: ReadableStream;
    closed: Promise<void>;
    cancel(reason: any): Promise<void>;
}

interface IReadableStreamDefaultReader extends ReadableStreamGenericReader {
    read(): Promise<ReadableStreamReadResult>;
    releaseLock(): void;
}

interface IReadableStreamBYOBReader extends ReadableStreamGenericReader {
    read(
        view: ArrayBufferView,
        options?: ReadableStreamBYOBReaderReadOptions,
    ): Promise<ReadableStreamReadResult>;
    releaseLock(): void;
}

type ReadableStreamReadResult = {
    value: any;
    done: boolean;
};

type QueueContainer<T = ValueWithSize<any>> = {
    [_queue]: Queue<T>;
    [_queueTotalSize]: number;
};

enum ReadableStreamReaderMode {
    "byob",
}

enum ReadableStreamType {
    "bytes",
}

const _highWaterMark = Symbol("highWaterMark");

function extractHighWaterMarkFromQueuingStrategyInit(
    init: QueueingStrategyInit,
) {
    if (typeof init !== "object") {
        throw new TypeError("Queuing strategy init must be an object");
    }

    const { highWaterMark } = init;
    if (highWaterMark === undefined) {
        throw new TypeError(
            "Queuing strategy must have a highWaterMark property",
        );
    }

    if (Number.isNaN(highWaterMark) || highWaterMark < 0) {
        throw new RangeError(
            "highWaterMark property of queuing strategy must be a non-negative number",
        );
    }

    return highWaterMark;
}

const _sizeFunction = (chunk: any): number => chunk.byteLength;

// https://streams.spec.whatwg.org/#bytelengthqueuingstrategy
// A common queuing strategy when dealing with bytes is to wait until
// the accumulated byteLength properties of the incoming chunks reaches a specified high-water mark.
// As such, this is provided as a built-in queuing strategy that can be used when constructing streams.
class ByteLengthQueuingStrategy implements QueuingStrategy {
    [_highWaterMark]: number;

    constructor(init: QueueingStrategyInit) {
        this[_highWaterMark] =
            extractHighWaterMarkFromQueuingStrategyInit(init);

        this.size = _sizeFunction;
    }

    size: QueuingStrategySizeCallback;

    get highWaterMark() {
        return this[_highWaterMark];
    }
}

// https://streams.spec.whatwg.org/#countqueuingstrategy
// A common queuing strategy when dealing with streams of generic objects is
// to simply count the number of chunks that have been accumulated so far,
// waiting until this number reaches a specified high-water mark. As such,
// this strategy is also provided out of the box.
class CountQueuingStrategy implements QueuingStrategy {
    [_highWaterMark]: number;

    constructor(init: QueueingStrategyInit) {
        this[_highWaterMark] =
            extractHighWaterMarkFromQueuingStrategyInit(init);

        this.size = () => 1;
    }

    size: QueuingStrategySizeCallback;

    get highWaterMark() {
        return this[_highWaterMark];
    }
}

const _cancelAlgorithm = Symbol("[cancelAlgorithm]");
const _closeRequested = Symbol("[closeRequested]");
const _pullAgain = Symbol("[pullAgain]");
const _pullAlgorithm = Symbol("[pullAlgorithm]");
const _pulling = Symbol("[pulling]");
const _queue = Symbol("[queue]");
const _queueTotalSize = Symbol("[queueTotalSize]");
const _started = Symbol("[started]");
const _strategyHWM = Symbol("[strategyHWM]");
const _strategySizeAlgorithm = Symbol("[strategySizeAlgorithm]");
const _stream = Symbol("[stream]");

const _state = Symbol("[state]");
const _controller = Symbol("[controller]");
const _detached = Symbol("[Detached]");
const _disturbed = Symbol("[disturbed]");
const _reader = Symbol("[reader]");
const _storedError = Symbol("[storedError]");

const _readRequests = Symbol("[readRequests]");
const _readIntoRequests = Symbol("[readIntoRequests]");
const _closedPromise = Symbol("[closedPromise]");

const _cancelSteps = Symbol("[cancelSteps]");
const _pullSteps = Symbol("[pullSteps]");
const _releaseSteps = Symbol("[releaseSteps]");
const _pendingPullIntos = Symbol("[pendingPullIntos]");
const _byobRequest = Symbol("[byobRequest]");
const _view = Symbol("[view]");
const _autoAllocateChunkSize = Symbol("[autoAllocateChunkSize]");
const _preventCancel = Symbol("[preventCancel]");

const _readable = "readable";

//x https://streams.spec.whatwg.org/#is-readable-stream-locked
function isReadableStreamLocked(stream: ReadableStream) {
    // 1. If stream.[[reader]] is undefined, return false.
    // 2. Return true.
    return stream[_reader] !== undefined;
}

// return true only if stream has a readable stream controller internal slot
// controller is initialized with null value
function isReadableStream(stream: ReadableStream) {
    return isObject(stream) && stream[_controller] !== undefined;
}

function isInReadableState(stream: ReadableStream) {
    return stream[_state] === "readable";
}

function closeStreamResource(
    resource: UnboundedReadableStreamResource,
    reader: ReadableStreamDefaultReader,
) {
    reader.cancel("Resource stream closed");
    op_close_unbounded_stream(resource);
}

function writeIntoResourceFromReadableStream(
    resource: UnboundedReadableStreamResource,
    reader: ReadableStreamDefaultReader,
    chunk: Uint8Array,
) {
    if (chunk.length === 0) {
        readIntoResourceFromReadableStream(resource, reader);
        return;
    }

    const output = op_write_sync_unbounded_stream(resource, chunk);
    if (output === -1) {
        closeStreamResource(resource, reader);
    } else if (output === -2) {
        // stream is full
        op_write_unbounded_stream(resource, chunk, (err, result) => {
            if (err) {
                closeStreamResource(resource, reader);
                return;
            }

            if (result === -1) {
                closeStreamResource(resource, reader);
            } else {
                readIntoResourceFromReadableStream(resource, reader);
            }
        });
    } else {
        readIntoResourceFromReadableStream(resource, reader);
    }
}

function readIntoResourceFromReadableStream(
    resource: UnboundedReadableStreamResource,
    reader: ReadableStreamDefaultReader,
) {
    // 1. Let promise be a new promise.
    // const promise = new Deferred<ReadableStreamReadResult>();
    // 2. Let readRequest be a new read request with the following items:
    const readRequest = {
        chunkSteps: (chunk: Uint8Array) => {
            writeIntoResourceFromReadableStream(resource, reader, chunk);
        },
        closeSteps: () => {
            op_close_unbounded_stream(resource);
        },
        errorSteps: (e: any) => {
            reader.cancel(e); // TODO: check if this is correct
            op_close_unbounded_stream(resource);
        },
    };

    // 3. Perform ! ReadableStreamDefaultReaderRead(this, readRequest).
    readableStreamDefaultReaderRead(reader, readRequest);
}

// Resource stream methods
function readableStreamResource(
    stream: ReadableStream,
    size?: number,
): UnboundedReadableStreamResource {
    const reader = acquireReadableStreamDefaultReader(stream);
    // const highWaterMark = stream[_controller][_strategyHWM] || size || 10;
    const resource = new UnboundedReadableStreamResource();

    readIntoResourceFromReadableStream(resource, reader);
    return resource;
}

//x https://streams.spec.whatwg.org/#readable-stream-close
function readableStreamClose(stream: ReadableStream) {
    // 1. Assert: stream.[[state]] is "readable".
    if (stream[_state] !== "readable")
        throw new TypeError("Stream is not readable");
    // 2. Set stream.[[state]] to "closed".
    stream[_state] = "closed";
    // 3. Let reader be stream.[[reader]].
    const reader = stream[_reader];
    // 4. If reader is undefined, return.
    if (reader === undefined) return;
    // 6. If reader implements ReadableStreamDefaultReader,
    if (isReadableStreamDefaultReader(reader)) {
        // 6.1. Let readRequests be reader.[[readRequests]].
        const readRequests: ReadRequest[] = reader[_readRequests];
        // 6.2. Set reader.[[readRequests]] to an empty list.
        reader[_readRequests] = [];
        // 6.3 For each readRequest of readRequests, Perform readRequest’s close steps.
        readRequests.forEach((readRequest) => readRequest.closeSteps());
    }
    // 5. Resolve reader.[[closedPromise]] with undefined.
    reader[_closedPromise].resolve(undefined);
}

//x https://streams.spec.whatwg.org/#readable-stream-cancel
const readableStreamCancel = (
    stream: ReadableStream,
    reason: any,
): Promise<void> => {
    // 1. Set stream.[[disturbed]] to true.
    stream[_disturbed] = true;
    // 2. If stream.[[state]] is "closed", return a promise resolved with undefined.
    if (stream[_state] === "closed") return Promise.resolve(undefined);
    // 3. If stream.[[state]] is "errored", return a promise rejected with stream.[[storedError]].
    if (stream[_state] === "errored")
        return Promise.reject(stream[_storedError]);
    // 4. Perform ! ReadableStreamClose(stream).
    readableStreamClose(stream);
    // 5. Let reader be stream.[[reader]].
    const reader = stream[_reader];
    // 6. If reader is not undefined and reader implements ReadableStreamBYOBReader,
    if (reader !== undefined && isReadableStreamBYOBReader(reader)) {
        // 6.1. Let readIntoRequests be reader.[[readIntoRequests]].
        const readIntoRequests = reader[_readIntoRequests];
        // 6.2. Set reader.[[readIntoRequests]] to an empty list.
        reader[_readIntoRequests] = [];
        // 6.3. For each readIntoRequest of readIntoRequests,
        readIntoRequests.forEach((readIntoRequest) => {
            // 6.3.1. Perform readIntoRequest’s close steps, given undefined.
            readIntoRequest.closeSteps(undefined);
        });
    }
    // 7. Let sourceCancelPromise be ! stream.[[controller]].[[CancelSteps]](reason).
    const sourceCancelPromise = stream[_controller][_cancelSteps](reason);
    // 8. Return the result of reacting to sourceCancelPromise with a fulfillment step that returns undefined.
    return sourceCancelPromise.then(() => undefined);
};

//x https://streams.spec.whatwg.org/#abstract-opdef-readablestreamdefaultreadererrorreadrequests
function readableStreamDefaultReaderErrorReadRequests(
    reader: ReadableStreamDefaultReader,
    e: any,
) {
    // 1. Let readRequests be reader.[[readRequests]].
    const readRequests = reader[_readRequests];
    // 2. Set reader.[[readRequests]] to a new empty list.
    reader[_readRequests] = [];
    // 3. For each readRequest of readRequests, Perform readRequest’s error steps, given e.
    readRequests.forEach((readRequest) => readRequest.errorSteps(e));
}

//x https://streams.spec.whatwg.org/#readable-stream-error
function readableStreamError(stream: ReadableStream, e: any) {
    // 1. Assert: stream.[[state]] is "readable".
    if (stream[_state] !== "readable")
        throw new TypeError("Stream is not readable");
    // 2. Set stream.[[state]] to "errored".
    stream[_state] = "errored";
    // 3. Set stream.[[storedError]] to e.
    stream[_storedError] = e;
    // 4. Let reader be stream.[[reader]].
    const reader = stream[_reader];
    // 5. If reader is undefined, return.
    if (reader === undefined) return;
    // 6. Reject reader.[[closedPromise]] with e.
    reader[_closedPromise].reject(e);
    // 7. Set reader.[[closedPromise]].[[PromiseIsHandled]] to true.
    // [[PromiseIsHandled]] Indicates whether the promise has ever had a fulfillment or rejection handler;
    // used in unhandled rejection tracking.
    reader[_closedPromise].promise.then(undefined, () => {});
    // 8. If reader implements ReadableStreamDefaultReader,
    if (isReadableStreamDefaultReader(reader)) {
        // 8.1 Perform ! ReadableStreamDefaultReaderErrorReadRequests(reader, e).
        readableStreamDefaultReaderErrorReadRequests(reader, e);
    } else {
        // 9.1 Otherwise, Assert: reader implements ReadableStreamBYOBReader.
        assert(
            isReadableStreamBYOBReader(reader),
            "Reader is not a BYOB reader",
        );
        // 9.2 Perform ! ReadableStreamBYOBReaderErrorReadIntoRequests(reader, e).
        readableStreamBYOBReaderErrorReadIntoRequests(reader, e);
    }
}

//x https://streams.spec.whatwg.org/#readable-stream-has-default-reader
function readableStreamHasDefaultReader(stream: ReadableStream) {
    // 1. Let reader be stream.[[reader]].
    const reader = stream[_reader];
    // 2. If reader is undefined, return false.
    if (reader === undefined) return false;
    // 3. If reader implements ReadableStreamDefaultReader, return true.
    if (isReadableStreamDefaultReader(reader)) return true;
    // 4. Return false.
    return false;
}

//x https://streams.spec.whatwg.org/#readable-stream-get-num-read-requests
function readableStreamGetNumReadRequests(stream: ReadableStream) {
    // 1. Assert: ! ReadableStreamHasDefaultReader(stream) is true.
    if (readableStreamHasDefaultReader(stream) !== true)
        throw new TypeError("Stream has no default reader");
    // 2. Return stream.[[reader]].[[readRequests]]'s size.
    return (stream[_reader] as ReadableStreamDefaultReader)[_readRequests]
        .length;
}

//x https://streams.spec.whatwg.org/#initialize-readable-stream
const initializeReadableStream = (stream: ReadableStream) => {
    // 1. Set stream.[[state]] to "readable".
    stream[_state] = "readable";
    // 2. Set stream.[[reader]] and stream.[[storedError]] to undefined.
    stream[_reader] = undefined;
    stream[_storedError] = undefined;
    // 3. Set stream.[[disturbed]] to false.
    stream[_disturbed] = false;
};

//x https://streams.spec.whatwg.org/#validate-and-normalize-high-water-mark
const extractHighWaterMark = (
    strategy: QueuingStrategy | undefined,
    defaultHWM: number,
) => {
    // 1. If strategy["highWaterMark"] does not exist, return defaultHWM.
    if (strategy?.highWaterMark === undefined) return defaultHWM;
    // 2. Let highWaterMark be strategy["highWaterMark"].
    const highWaterMark = strategy.highWaterMark;
    // 3. If highWaterMark is NaN or highWaterMark < 0, throw a RangeError exception.
    if (Number.isNaN(highWaterMark) || highWaterMark < 0)
        throw new RangeError("High water mark must be a non-negative number");
    // 4. Return highWaterMark.
    return highWaterMark;
};

//x https://streams.spec.whatwg.org/#set-up-readable-byte-stream-controller
const setUpReadableByteStreamController = (
    stream: ReadableStream,
    controller: ReadableByteStreamController,
    startAlgorithm: () => void,
    pullAlgorithm: () => Promise<void>,
    cancelAlgorithm: (reason: any) => Promise<void>,
    highWaterMark: number,
    autoAllocateChunkSize?: number,
) => {
    // 1. Assert: stream.[[controller]] is undefined.
    if (stream[_controller] !== undefined)
        throw new TypeError("Stream controller is not undefined");
    // 2. If autoAllocateChunkSize is not undefined,
    if (autoAllocateChunkSize !== undefined) {
        // 2.1. Assert: ! IsInteger(autoAllocateChunkSize) is true.
        assert(
            Number.isInteger(autoAllocateChunkSize),
            "Auto allocate chunk size must be an integer",
        );
        // 2.2. Assert: autoAllocateChunkSize is positive.
        if (autoAllocateChunkSize <= 0)
            throw new RangeError(
                "Auto allocate chunk size must be greater than 0",
            );
    }
    // 3. Set controller.[[stream]] to stream.
    controller[_stream] = stream;
    // 4. Set controller.[[pullAgain]] and controller.[[pulling]] to false.
    controller[_pullAgain] = false;
    controller[_pulling] = false;
    // 5. Set controller.[[byobRequest]] to null.
    controller[_byobRequest] = null;
    // 6. Perform ! ResetQueue(controller).
    resetQueue(controller);
    // 7. Set controller.[[closeRequested]] and controller.[[started]] to false.
    controller[_closeRequested] = false;
    controller[_started] = false;
    // 8. Set controller.[[strategyHWM]] to highWaterMark.
    controller[_strategyHWM] = highWaterMark;
    // 9. Set controller.[[pullAlgorithm]] to pullAlgorithm.
    controller[_pullAlgorithm] = pullAlgorithm;
    // 10. Set controller.[[cancelAlgorithm]] to cancelAlgorithm.
    controller[_cancelAlgorithm] = cancelAlgorithm;
    // 11. Set controller.[[autoAllocateChunkSize]] to autoAllocateChunkSize.
    controller[_autoAllocateChunkSize] = autoAllocateChunkSize;
    // 12. Set controller.[[pendingPullIntos]] to a new empty list.
    controller[_pendingPullIntos] = [];
    // 13. Set stream.[[controller]] to controller.
    stream[_controller] = controller;
    // 14. Let startResult be the result of performing startAlgorithm.
    const startResult = startAlgorithm();
    // 15. Let startPromise be a promise resolved with startResult.
    const startPromise = Promise.resolve(startResult);
    // 16. Upon fulfillment of startPromise,
    startPromise.then(
        () => {
            // 16.1. Set controller.[[started]] to true.
            controller[_started] = true;
            // 16.2. Assert: controller.[[pulling]] is false.
            assert(controller[_pulling] === false, "Controller is pulling");
            // 16.3. Assert: controller.[[pullAgain]] is false.
            assert(
                controller[_pullAgain] === false,
                "Controller is pulling again",
            );
            // 16.4. Perform ! ReadableByteStreamControllerCallPullIfNeeded(controller).
            readableByteStreamControllerCallPullIfNeeded(controller);
        },
        // 17. Upon rejection of startPromise with reason r,
        (r) => {
            // 17.1. Perform ! ReadableByteStreamControllerError(controller, r).
            readableByteStreamControllerError(controller, r);
        },
    );
};

//x https://streams.spec.whatwg.org/#set-up-readable-byte-stream-controller-from-underlying-source
const setUpReadableByteStreamControllerFromUnderlyingSource = (
    stream: ReadableStream,
    underlyingSource: UnderlyingSource,
    underlyingSourceDict: UnderlyingSource,
    highWaterMark: number,
) => {
    // 1. Let controller be a new ReadableByteStreamController.
    const controller = new ReadableByteStreamController();
    // 2. Let startAlgorithm be an algorithm that returns undefined.
    let startAlgorithm = () => undefined;
    // 3. Let pullAlgorithm be an algorithm that returns a promise resolved with undefined.
    let pullAlgorithm = () => Promise.resolve(undefined);
    // 4. Let cancelAlgorithm be an algorithm that returns a promise resolved with undefined.
    let cancelAlgorithm = (_: any) => Promise.resolve(undefined);
    // 5. If underlyingSourceDict["start"] exists, then set startAlgorithm to an algorithm which returns the result of invoking underlyingSourceDict["start"] with argument list « controller » and callback this value underlyingSource.
    if (underlyingSourceDict.start)
        startAlgorithm = () =>
            underlyingSourceDict.start!.call(underlyingSource, controller);
    // 6. If underlyingSourceDict["pull"] exists, then set pullAlgorithm to an algorithm which returns the result of invoking underlyingSourceDict["pull"] with argument list « controller » and callback this value underlyingSource.
    if (underlyingSourceDict.pull)
        pullAlgorithm = async () =>
            await underlyingSourceDict.pull!.call(underlyingSource, controller);
    // 7. If underlyingSourceDict["cancel"] exists, then set cancelAlgorithm to an algorithm which takes an argument reason and returns the result of invoking underlyingSourceDict["cancel"] with argument list « reason » and callback this value underlyingSource.
    if (underlyingSourceDict.cancel)
        cancelAlgorithm = async (reason: any) =>
            await underlyingSourceDict.cancel!.call(underlyingSource, reason);
    // 8. Let autoAllocateChunkSize be underlyingSourceDict["autoAllocateChunkSize"], if it exists, or undefined otherwise.
    const autoAllocateChunkSize = underlyingSourceDict.autoAllocateChunkSize;
    // 9. If autoAllocateChunkSize is 0, then throw a TypeError exception.
    if (autoAllocateChunkSize === 0)
        throw new TypeError("Auto allocate chunk size must be greater than 0");
    // 10. Perform ? SetUpReadableByteStreamController(stream, controller, startAlgorithm, pullAlgorithm, cancelAlgorithm, highWaterMark, autoAllocateChunkSize).
    setUpReadableByteStreamController(
        stream,
        controller,
        startAlgorithm,
        pullAlgorithm,
        cancelAlgorithm,
        highWaterMark,
        autoAllocateChunkSize,
    );
};

//x https://streams.spec.whatwg.org/#make-size-algorithm-from-size-function
const extractSizeAlgorithm = (strategy: QueuingStrategy | undefined) => {
    // 1. If strategy["size"] does not exist, return an algorithm that returns 1.
    if (strategy?.size === undefined) return () => 1;
    // 2. Return an algorithm that performs the following steps, taking a chunk argument:
    // 2.1 Return the result of invoking strategy["size"] with argument list « chunk ».
    return (chunk: any) => strategy.size!(chunk);
};

//x https://streams.spec.whatwg.org/#set-up-readable-stream-default-controller
const setUpReadableStreamDefaultController = (
    stream: ReadableStream,
    controller: ReadableStreamDefaultController,
    startAlgorithm: () => void,
    pullAlgorithm: () => Promise<void>,
    cancelAlgorithm: (reason: any) => Promise<void>,
    highWaterMark: number,
    sizeAlgorithm: QueuingStrategySizeCallback,
) => {
    // 1. Assert: stream.[[controller]] is undefined.
    if (stream[_controller] !== undefined)
        throw new TypeError("Stream controller is not undefined");
    // 2. Set controller.[[stream]] to stream.
    controller[_stream] = stream;
    // 3. Perform ! ResetQueue(controller).
    resetQueue(controller);
    // 4. Set controller.[[started]], controller.[[closeRequested]], controller.[[pullAgain]], and controller.[[pulling]] to false.
    controller[_started] = false;
    controller[_closeRequested] = false;
    controller[_pullAgain] = false;
    controller[_pulling] = false;
    // 5. Set controller.[[strategySizeAlgorithm]] to sizeAlgorithm and controller.[[strategyHWM]] to highWaterMark.
    controller[_strategySizeAlgorithm] = sizeAlgorithm;
    controller[_strategyHWM] = highWaterMark;
    // 6. Set controller.[[pullAlgorithm]] to pullAlgorithm.
    controller[_pullAlgorithm] = pullAlgorithm;
    // 7. Set controller.[[cancelAlgorithm]] to cancelAlgorithm.
    controller[_cancelAlgorithm] = cancelAlgorithm;
    // 8. Set stream.[[controller]] to controller.
    stream[_controller] = controller;
    // 9. Let startResult be the result of performing startAlgorithm. (This might throw an exception.)
    const startResult = startAlgorithm();
    // 10. Let startPromise be a promise resolved with startResult.
    const startPromise = Promise.resolve(startResult);
    // 11. Upon fulfillment of startPromise,
    startPromise.then(
        () => {
            // 11.1. Set controller.[[started]] to true.
            controller[_started] = true;
            // 11.2. Assert: controller.[[pulling]] is false.
            assert(controller[_pulling] === false, "Controller is pulling");
            // 11.3. Assert: controller.[[pullAgain]] is false.
            assert(
                controller[_pullAgain] === false,
                "Controller is pulling again",
            );
            // 11.4. Perform ! ReadableStreamDefaultControllerCallPullIfNeeded(controller).
            readableStreamDefaultControllerCallPullIfNeeded(controller);
        },
        // 12. Upon rejection of startPromise with reason r,
        (r) => {
            // 12.1. Perform ! ReadableStreamDefaultControllerError(controller, r).
            readableStreamDefaultControllerError(controller, r);
        },
    );
};

//x https://streams.spec.whatwg.org/#set-up-readable-stream-default-controller-from-underlying-source
const setUpReadableStreamDefaultControllerFromUnderlyingSource = (
    stream: ReadableStream,
    underlyingSource: UnderlyingSource,
    underlyingSourceDict: UnderlyingSource,
    highWaterMark: number,
    sizeAlgorithm: QueuingStrategySizeCallback,
) => {
    // 1. Let controller be a new ReadableStreamDefaultController.
    const controller = new ReadableStreamDefaultController();
    // 2. Let startAlgorithm be an algorithm that returns undefined.
    let startAlgorithm = () => undefined;
    // 3. Let pullAlgorithm be an algorithm that returns a promise resolved with undefined.
    let pullAlgorithm = () => Promise.resolve(undefined);
    // 4. Let cancelAlgorithm be an algorithm that returns a promise resolved with undefined.
    let cancelAlgorithm = (_: any) => Promise.resolve(undefined);
    // 5. If underlyingSourceDict["start"] exists, then set startAlgorithm to an algorithm which returns the result of invoking underlyingSourceDict["start"] with argument list « controller » and callback this value underlyingSource.
    if (underlyingSourceDict.start)
        startAlgorithm = () =>
            underlyingSourceDict.start!.call(underlyingSource, controller);
    // 6. If underlyingSourceDict["pull"] exists, then set pullAlgorithm to an algorithm which returns the result of invoking underlyingSourceDict["pull"] with argument list « controller » and callback this value underlyingSource.
    if (underlyingSourceDict.pull)
        pullAlgorithm = async () =>
            await underlyingSourceDict.pull!.call(underlyingSource, controller);
    // 7. If underlyingSourceDict["cancel"] exists, then set cancelAlgorithm to an algorithm which takes an argument reason and returns the result of invoking underlyingSourceDict["cancel"] with argument list « reason » and callback this value underlyingSource.
    if (underlyingSourceDict.cancel)
        cancelAlgorithm = async (reason: any) =>
            await underlyingSourceDict.cancel!.call(underlyingSource, reason);
    // 8. Perform ? SetUpReadableStreamDefaultController(stream, controller, startAlgorithm, pullAlgorithm, cancelAlgorithm, highWaterMark, sizeAlgorithm).
    setUpReadableStreamDefaultController(
        stream,
        controller,
        startAlgorithm,
        pullAlgorithm,
        cancelAlgorithm,
        highWaterMark,
        sizeAlgorithm,
    );
};

//x https://streams.spec.whatwg.org/#create-readable-stream
const createReadableStream = <T>(
    startAlgorithm: () => void,
    pullAlgorithm: () => Promise<void>,
    cancelAlgorithm: (reason: any) => Promise<void>,
    highWaterMark: number = 1,
    sizeAlgorithm: QueuingStrategySizeCallback<T> = () => 1,
): ReadableStream => {
    // 1. If highWaterMark was not passed, set it to 1.
    // 2. If sizeAlgorithm was not passed, set it to an algorithm that returns 1.
    // 3. Assert: ! IsNonNegativeNumber(highWaterMark) is true.
    assert(highWaterMark >= 0, "High water mark must be a non-negative number");
    // 4. Let stream be a new ReadableStream.
    const stream = new ReadableStream(_internalUnderlyingSource as any);
    // 5. Perform ! InitializeReadableStream(stream).
    initializeReadableStream(stream);
    // 6. Let controller be a new ReadableStreamDefaultController.
    const controller = new ReadableStreamDefaultController();
    // 7. Perform ? SetUpReadableStreamDefaultController(stream, controller, startAlgorithm, pullAlgorithm, cancelAlgorithm, highWaterMark, sizeAlgorithm).
    setUpReadableStreamDefaultController(
        stream,
        controller,
        startAlgorithm,
        pullAlgorithm,
        cancelAlgorithm,
        highWaterMark,
        sizeAlgorithm,
    );
    // 8. Return stream.
    return stream;
};

//x https://streams.spec.whatwg.org/#readable-stream-from-iterable
const readableStreamFromIterable = <T>(
    iterable: Iterable<T> | AsyncIterable<T>,
): ReadableStream => {
    // 1. Let stream be undefined.
    let stream: ReadableStream | undefined;
    // 2. Let iteratorRecord be ? GetIterator(asyncIterable, async).
    const iteratorRecord = getIterator(iterable);
    // 3. Let startAlgorithm be an algorithm that returns undefined.
    let startAlgorithm = () => undefined;
    // 4. Let pullAlgorithm be the following steps:
    const pullAlgorithm = async () => {
        // 4.1. Let nextResult be IteratorNext(iteratorRecord).
        let nextResult: IteratorResult<any, any>;
        try {
            nextResult = await iteratorRecord.next();
        } catch (error) {
            // 4.2. If nextResult is an abrupt completion, return a promise rejected with nextResult.[[Value]].
            return Promise.reject(error);
        }
        // 4.3. Let nextPromise be a promise resolved with nextResult.[[Value]].
        const nextPromise = Promise.resolve(nextResult);
        // 4.4. Return the result of reacting to nextPromise with the following fulfillment steps, given iterResult:
        return await nextPromise.then((iterResult) => {
            // 4.4.1. If Type(iterResult) is not Object, throw a TypeError.
            if (!isObject(iterResult))
                throw new TypeError("Iterator result must be an object");
            // 4.4.2. Let done be ? IteratorComplete(iterResult).
            const done = iterResult.done;
            // 4.4.3. If done is true: Perform ! ReadableStreamDefaultControllerClose(stream.[[controller]]).
            if (done) {
                readableStreamDefaultControllerClose(
                    stream![_controller] as ReadableStreamDefaultController,
                );
            } else {
                // 4.4.4 Let value be ? IteratorValue(iterResult).
                const value = iterResult.value;
                // 4.4.5. Perform ! ReadableStreamDefaultControllerEnqueue(stream.[[controller]], value).
                readableStreamDefaultControllerEnqueue(
                    stream![_controller] as ReadableStreamDefaultController,
                    value,
                );
            }
        });
    };
    // 5. Let cancelAlgorithm be the following steps, given reason:
    const cancelAlgorithm = async (reason: any) => {
        // 5.1. Let iterator be iteratorRecord.[[Iterator]].
        const iterator = iteratorRecord;
        // 5.2. Let returnMethod be GetMethod(iterator, "return").
        // const returnMethod = iterator.return;
        // 5.3. If returnMethod is an abrupt completion, return a promise rejected with returnMethod.[[Value]].
        // 5.4 If returnMethod.[[Value]] is undefined, return a promise resolved with undefined.
        if (iterator.return === undefined || iterator.return === null)
            return undefined;
        // 5.5. Let returnResult be Call(returnMethod.[[Value]], iterator, « reason »).
        const returnResult = await iterator.return(reason);
        // 5.6. If returnResult is an abrupt completion, return a promise rejected with returnResult.[[Value]].
        // 5.7. Let returnPromise be a promise resolved with returnResult.[[Value]].
        // 5.8. Return the result of reacting to returnPromise with the following fulfillment steps, given iterResult:
        // 5.8.1. If Type(iterResult) is not Object, throw a TypeError.
        if (!isObject(returnResult))
            throw new TypeError("Iterator result must be an object");
        // 5.8.2. Return undefined.
        return undefined;
    };
    // 6. Set stream to ! CreateReadableStream(startAlgorithm, pullAlgorithm, cancelAlgorithm, 0).
    stream = createReadableStream(
        startAlgorithm,
        pullAlgorithm,
        cancelAlgorithm,
        0,
    );
    // 7. Return stream.
    return stream;
};

//x https://streams.spec.whatwg.org/#acquire-readable-stream-reader
const acquireReadableStreamDefaultReader = (
    stream: ReadableStream,
): ReadableStreamDefaultReader => {
    // 1. Let reader be a new ReadableStreamDefaultReader.
    // 2. Perform ? SetUpReadableStreamDefaultReader(reader, stream).
    // 3. Return reader.
    const reader = new ReadableStreamDefaultReader(stream);
    return reader;
};

//x https://streams.spec.whatwg.org/#acquire-readable-stream-byob-reader
const acquireReadableStreamBYOBReader = (
    stream: ReadableStream,
): ReadableStreamBYOBReader => {
    // 1. Let reader be a new ReadableStreamBYOBReader.
    // 2. Perform ? SetUpReadableStreamBYOBReader(reader, stream).
    // 3. Return reader.
    const reader = new ReadableStreamBYOBReader(stream);
    return reader;
};

const RedableStreamAsyncIteratorPrototype = Object.setPrototypeOf(
    {
        next() {
            // 1. Let reader be iterator’s reader.
            const reader = this[_reader];
            // 2. Assert: reader.[[stream]] is not undefined.
            assert(reader[_stream] !== undefined, "Stream is undefined");
            // 3. Let promise be a new promise.
            const promise = new Deferred<void>();
            // 4. Let readRequest be a new read request with the following items:
            const readRequest: ReadRequest = {
                // 4.1 chunk steps, given chunk, Resolve promise with chunk.
                chunkSteps: (chunk) =>
                    promise.resolve({ value: chunk, done: false }),
                // 4.2 close steps.
                closeSteps() {
                    // 4.2.1. Perform ! ReadableStreamDefaultReaderRelease(reader).
                    readableStreamDefaultReaderRelease(reader);
                    // 4.2.2. Resolve promise with end of iteration.
                    promise.resolve({ value: undefined, done: true });
                },
                // 4.3 error steps, given e
                errorSteps(e) {
                    // 4.3.1. Perform ! ReadableStreamDefaultReaderRelease(reader).
                    readableStreamDefaultReaderRelease(reader);
                    // 4.3.2. Reject promise with e.
                    promise.reject(e);
                },
            };
            // 5. Perform ! ReadableStreamDefaultReaderRead(reader, readRequest).
            readableStreamDefaultReaderRead(reader, readRequest);
            // 6. Return promise.
            return promise.promise;
        },
        return() {
            // 1. Let reader be iterator’s reader.
            const reader = this[_reader];
            // 2. Assert: reader.[[stream]] is not undefined.
            assert(reader[_stream] !== undefined, "Stream is undefined");
            // 3. Assert: reader.[[readRequests]] is empty, as the async iterator machinery guarantees that any previous calls to next() have settled before this is called.
            assert(
                reader[_readRequests].length === 0,
                "Read requests must be empty",
            );
            // 4. If iterator’s prevent cancel is false:
            if (this[_preventCancel] === false) {
                // 4.1. Let result be ! ReadableStreamReaderGenericCancel(reader, arg).
                const result = readableStreamReaderGenericCancel(
                    reader,
                    undefined,
                );
                // 4.2. Perform ! ReadableStreamDefaultReaderRelease(reader).
                readableStreamDefaultReaderRelease(reader);
                // 4.3 Return result.
                return result;
            }
            // 5. Perform ! ReadableStreamDefaultReaderRelease(reader).
            readableStreamDefaultReaderRelease(reader);
            // 6. Return a promise resolved with undefined.
            return Promise.resolve({ value: undefined, done: true });
        },
    },
    AsyncIteratorPrototype,
);

const isDisturbed = (stream: ReadableStream) => stream[_disturbed];
const isErrored = (stream: ReadableStream) => stream[_state] === "errored";

const readableStreamEnqueue = (
    stream: ReadableStream,
    chunk: ArrayBufferView,
) => {
    const controller = stream[_controller];
    // 1. If stream.[[controller]] implements ReadableStreamDefaultController,
    if (controller instanceof ReadableStreamDefaultController) {
        // 1.1. Perform ! ReadableStreamDefaultControllerEnqueue(stream.[[controller]], chunk).
        readableStreamDefaultControllerEnqueue(controller, chunk);
    } else {
        // 2. Otherwise,
        // 2.1. Assert: stream.[[controller]] implements ReadableByteStreamController.
        assert(
            controller instanceof ReadableByteStreamController,
            "Controller must be a ReadableByteStreamController",
        );
        // 2.2. Assert: chunk is an ArrayBufferView.
        assert(ArrayBuffer.isView(chunk), "Chunk must be an ArrayBufferView");
        // 2.3. Let byobView be the current BYOB request view for stream.
        const byobView = controller[_byobRequest]
            ? controller[_byobRequest].view
            : null;
        // 2.4. If byobView is non-null, and chunk.[[ViewedArrayBuffer]] is byobView.[[ViewedArrayBuffer]], then:
        if (
            byobView !== null &&
            chunk.buffer === byobView.buffer &&
            chunk.byteOffset === byobView.byteOffset &&
            chunk.byteLength <= byobView.byteLength
        ) {
            // 2.4.1. Assert: chunk.[[ByteOffset]] is byobView.[[ByteOffset]].
            assert(
                chunk.byteOffset === byobView.byteOffset,
                "Byte offset must be the same",
            );
            // 2.4.2. Assert: chunk.[[ByteLength]] ≤ byobView.[[ByteLength]].
            assert(
                chunk.byteLength <= byobView.byteLength,
                "Byte length must be less than or equal to the view's byte length",
            );
            // 2.4.1. Perform ? readableByteStreamControllerRespond(stream.[[controller]], chunk.[[ByteLength]]).
            readableByteStreamControllerRespond(controller, chunk.byteLength);
        } else {
            // 2.5. Otherwise, perform ? ReadableByteStreamControllerEnqueue(stream.[[controller]], chunk).
            readableByteStreamControllerEnqueue(controller, chunk);
        }
    }
};

const _internalUnderlyingSource = Symbol("[internalUnderlyingSource]");

class ReadableStream {
    [_controller]:
        | ReadableStreamDefaultController
        | ReadableByteStreamController;
    [_detached]: boolean;
    [_disturbed]: boolean;
    [_reader]:
        | ReadableStreamDefaultReader
        | ReadableStreamBYOBReader
        | undefined;
    [_state]: ReadableStreamState;
    [_storedError]: any;

    constructor(
        underlyingSource?: UnderlyingSource | null,
        strategy?: QueuingStrategy,
    ) {
        if ((underlyingSource as any) === _internalUnderlyingSource) {
            return;
        }
        // 1. If underlyingSource is missing, set it to null.
        if (underlyingSource === undefined) underlyingSource = null;
        // 2. Let underlyingSourceDict be underlyingSource, converted to an IDL value of type UnderlyingSource.
        // FIXME: This is a hack to make the code compile. We need to implement the conversion from JS object to IDL value.
        const underlyingSourceDict =
            (underlyingSource as UnderlyingSource) || {};
        // 3. Perform ! InitializeReadableStream(this).
        initializeReadableStream(this);
        // 4. If underlyingSourceDict["type"] is "bytes":
        if (underlyingSourceDict?.type === "bytes") {
            // 4.1. If strategy["size"] exists, throw a RangeError exception.
            if (strategy?.size !== undefined)
                throw new RangeError("Size property is not allowed");
            // 4.2. Let highWaterMark be ? ExtractHighWaterMark(strategy, 0).
            const highWaterMark = extractHighWaterMark(strategy, 0);
            // 4.3 Perform ? SetUpReadableByteStreamControllerFromUnderlyingSource(this, underlyingSource, underlyingSourceDict, highWaterMark).
            setUpReadableByteStreamControllerFromUnderlyingSource(
                this,
                underlyingSource!,
                underlyingSourceDict,
                highWaterMark,
            );
        } else {
            // 5. Otherwise,
            // 5.1. Assert: underlyingSourceDict["type"] does not exist.
            assert(
                underlyingSourceDict?.type === undefined,
                "Type property must not exist",
            );
            // 5.2. Let sizeAlgorithm be ! ExtractSizeAlgorithm(strategy).
            const sizeAlgorithm = extractSizeAlgorithm(strategy);
            // 5.3. Let highWaterMark be ? ExtractHighWaterMark(strategy, 1).
            const highWaterMark = extractHighWaterMark(strategy, 1);
            // 5.4. Perform ? SetUpReadableStreamDefaultControllerFromUnderlyingSource(this, underlyingSource, underlyingSourceDict, highWaterMark, sizeAlgorithm).
            setUpReadableStreamDefaultControllerFromUnderlyingSource(
                this,
                underlyingSource!,
                underlyingSourceDict,
                highWaterMark,
                sizeAlgorithm,
            );
        }
    }

    get locked() {
        //x
        if (!isReadableStream(this)) return false;

        return isReadableStreamLocked(this);
    }

    cancel(reason: any): Promise<void> {
        //x
        if (!isReadableStream(this))
            return Promise.reject(
                new TypeError(
                    "ReadableStream.prototype.cancel can only be used on a ReadableStream instance",
                ),
            );
        // 1. If ! IsReadableStreamLocked(this) is true, return a promise rejected with a TypeError exception.
        if (isReadableStreamLocked(this))
            return Promise.reject(new TypeError("Stream is locked"));
        // 2. Return ! ReadableStreamCancel(this, reason).
        return readableStreamCancel(this, reason);
    }

    static from<T>(
        asyncIterable: Iterable<T> | AsyncIterable<T>,
    ): ReadableStream {
        //x
        // 1. Return ? ReadableStreamFromIterable(asyncIterable).
        return readableStreamFromIterable(asyncIterable);
    }

    getReader<
        T = ReadableStreamDefaultReader | ReadableStreamBYOBReader,
    >(options?: { mode: "byob" }): T {
        //x
        assert(
            isReadableStream(this),
            "ReadableStream.prototype.getReader can only be used on a ReadableStream instance",
        );
        // 1. If options["mode"] does not exist, return ? AcquireReadableStreamDefaultReader(this).
        if (options?.mode === undefined)
            return acquireReadableStreamDefaultReader(this) as T;
        // 2. Assert: options["mode"] is "byob".
        assert(options.mode === "byob", 'Mode must be "byob"');
        // 3. Return ? AcquireReadableStreamBYOBReader(this).
        return acquireReadableStreamBYOBReader(this) as T;
    }

    values(args: { preventCancel?: boolean } = {}): AsyncIterableIterator<any> {
        //x
        assert(
            isReadableStream(this),
            "ReadableStream.prototype.values can only be used on a ReadableStream instance",
        );
        // 1. Let reader be ? AcquireReadableStreamDefaultReader(stream).
        const reader = acquireReadableStreamDefaultReader(this);
        // 2. Set iterator’s reader to reader.
        const iterator = Object.create(RedableStreamAsyncIteratorPrototype);
        iterator[_reader] = reader;
        // 3. Let preventCancel be args[0]["preventCancel"].
        // 4. Set iterator’s prevent cancel to preventCancel.
        iterator[_preventCancel] = args.preventCancel;
        return iterator;
    }

    [Symbol.asyncIterator]() {
        //x
        return this.values();
    }
}

// ---------------------------------------------------------------|
//              ReadableStreamDefaultReader                       |
// ---------------------------------------------------------------|
const isReadableStreamDefaultReader = (
    reader: any,
): reader is ReadableStreamDefaultReader => {
    return isObject(reader) && !!reader[_readRequests];
};

//x https://streams.spec.whatwg.org/#readable-stream-default-reader-read
function readableStreamDefaultReaderRead(
    reader: ReadableStreamDefaultReader,
    readRequest: ReadRequest,
) {
    // 1. Let stream be reader.[[stream]].
    const stream = reader[_stream];
    // 2. Assert: stream is not undefined.
    if (stream === undefined) throw new TypeError("Stream is undefined");
    // 3. Set stream.[[disturbed]] to true.
    stream[_disturbed] = true;
    // 4. If stream.[[state]] is "closed", perform readRequest’s close steps.
    if (stream[_state] === "closed") {
        readRequest.closeSteps();
    } else if (stream[_state] === "errored") {
        // 5. Otherwise, if stream.[[state]] is "errored", perform readRequest’s error steps given stream.[[storedError]].
        readRequest.errorSteps(stream[_storedError]);
    } else {
        // 6. Otherwise,
        // 6.1. Assert: stream.[[state]] is "readable".
        assert(stream[_state] === "readable", "Stream is not readable");
        // 6.2. Perform ! stream.[[controller]].[[PullSteps]](readRequest).
        stream[_controller][_pullSteps](readRequest);
    }
}

//x https://streams.spec.whatwg.org/#readable-stream-reader-generic-cancel
function readableStreamReaderGenericCancel(
    reader: ReadableStreamGenericReader,
    reason: any,
) {
    // 1. Let stream be reader.[[stream]].
    const stream = reader[_stream];
    // 2. Assert: stream is not undefined.
    if (stream === undefined) throw new TypeError("Stream is undefined");
    // 3. Return ! ReadableStreamCancel(stream, reason).
    return readableStreamCancel(stream, reason);
}

//x https://streams.spec.whatwg.org/#readable-stream-reader-generic-release
function readableStreamReaderGenericRelease(
    reader: ReadableStreamGenericReader,
) {
    // 1. Let stream be reader.[[stream]].
    const stream = reader[_stream];
    // 2. Assert: stream is not undefined.
    if (stream === undefined) throw new TypeError("Stream is undefined");
    // 3. Assert: stream.[[reader]] is reader.
    if (stream[_reader] !== reader)
        throw new TypeError("Stream reader is not the reader");
    // 4. If stream.[[state]] is "readable", reject reader.[[closedPromise]] with a TypeError exception.
    if (stream[_state] === "readable") {
        reader[_closedPromise].reject(new TypeError("Stream is readable"));
    } else {
        // 5. Otherwise, set reader.[[closedPromise]] to a promise rejected with a TypeError exception.
        reader[_closedPromise] = new Deferred();
        reader[_closedPromise].reject(new TypeError("Stream is readable"));
    }
    // 6. Set reader.[[closedPromise]].[[PromiseIsHandled]] to true.
    reader[_closedPromise].promise.then(undefined, () => {});
    // 7. Perform ! stream.[[controller]].[[ReleaseSteps]]().
    stream[_controller][_releaseSteps]();
    // 8. Set stream.[[reader]] to undefined.
    stream[_reader] = undefined;
    // 9. Set reader.[[stream]] to undefined.
    reader[_stream] = undefined;
}

//x https://streams.spec.whatwg.org/#abstract-opdef-readablestreamdefaultreaderrelease
function readableStreamDefaultReaderRelease(
    reader: ReadableStreamDefaultReader,
) {
    // 1. Perform ! ReadableStreamReaderGenericRelease(reader).
    readableStreamReaderGenericRelease(reader);
    // 2. Let e be a new TypeError exception.
    const e = new TypeError("Reader is released");
    // 3. Perform ! ReadableStreamDefaultReaderErrorReadRequests(reader, e).
    readableStreamDefaultReaderErrorReadRequests(reader, e);
}

//x https://streams.spec.whatwg.org/#readable-stream-reader-generic-initialize
function readableStreamReaderGenericInitialize(
    reader: ReadableStreamGenericReader,
    stream: ReadableStream,
) {
    // 1. Set reader.[[stream]] to stream.
    reader[_stream] = stream;
    // 2. Set stream.[[reader]] to reader.
    stream[_reader] = reader as any;
    // 3. If stream.[[state]] is "readable",
    if (stream[_state] === "readable") {
        // 3.1. Set reader.[[closedPromise]] to a new promise.
        reader[_closedPromise] = new Deferred();
    } else if (stream[_state] === "closed") {
        // 4. Otherwise, if stream.[[state]] is "closed",
        // 4.1 Set reader.[[closedPromise]] to a promise resolved with undefined.
        reader[_closedPromise] = new Deferred();
        reader[_closedPromise].resolve(undefined);
    } else {
        // 5. Otherwise,
        // 5.1 Assert: stream.[[state]] is "errored".
        assert(stream[_state] === "errored", "Stream is not errored");
        // 5.2 Set reader.[[closedPromise]] to a promise rejected with stream.[[storedError]].
        reader[_closedPromise] = new Deferred();
        reader[_closedPromise].reject(stream[_storedError]);
        // 5.3 Set reader.[[closedPromise]].[[PromiseIsHandled]] to true.
        reader[_closedPromise].promise.then(undefined, () => {});
    }
}

//x https://streams.spec.whatwg.org/#set-up-readable-stream-default-reader
function setUpReadableStreamDefaultReader(
    reader: ReadableStreamDefaultReader,
    stream: ReadableStream,
) {
    // 1. If ! IsReadableStreamLocked(stream) is true, throw a TypeError exception.
    if (isReadableStreamLocked(stream)) throw new TypeError("Stream is locked");
    // 2. Perform ! ReadableStreamReaderGenericInitialize(reader, stream).
    readableStreamReaderGenericInitialize(reader, stream);
    // 3. Set reader.[[readRequests]] to a new empty list.
    reader[_readRequests] = [];
}

//x https://streams.spec.whatwg.org/#readablestreamdefaultreader
class ReadableStreamDefaultReader implements IReadableStreamDefaultReader {
    [_closedPromise]: Deferred;
    [_stream]: ReadableStream;
    [_readRequests]: ReadRequest[];

    constructor(stream: ReadableStream) {
        //x
        if (!isReadableStream(stream)) {
            throw new TypeError(
                "ReadableStreamDefaultReader can only be constructed with a ReadableStream instance",
            );
        }

        setUpReadableStreamDefaultReader(this, stream);
    }

    get closed() {
        //x
        if (!isReadableStreamDefaultReader(this)) {
            return Promise.reject(
                new TypeError(
                    "ReadableStreamDefaultReader.prototype.closed can only be used on a ReadableStreamDefaultReader",
                ),
            );
        }

        return this[_closedPromise].promise;
    }

    cancel(reason: any): Promise<void> {
        //x
        if (!isReadableStreamDefaultReader(this)) {
            return Promise.reject(
                new TypeError(
                    "ReadableStreamDefaultReader.prototype.cancel can only be used on a ReadableStreamDefaultReader",
                ),
            );
        }

        // 1. If this.[[stream]] is undefined, return a promise rejected with a TypeError exception.
        if (this[_stream] === undefined)
            return Promise.reject(
                new TypeError("Reader is not associated with a stream"),
            );

        // 2. Return ! ReadableStreamReaderGenericCancel(this, reason).
        return readableStreamReaderGenericCancel(this, reason);
    }

    read() {
        //x
        if (!isReadableStreamDefaultReader(this)) {
            return Promise.reject(
                new TypeError(
                    "ReadableStreamDefaultReader.prototype.read can only be used on a ReadableStreamDefaultReader",
                ),
            );
        }

        // 1. If this.[[stream]] is undefined, return a promise rejected with a TypeError exception.
        if (this[_stream] === undefined)
            return Promise.reject(
                new TypeError("Reader is not associated with a stream"),
            );

        // 2. Let promise be a new promise.
        const promise = new Deferred<ReadableStreamReadResult>();

        // 3. Let readRequest be a new read request with the following items:
        const readRequest = {
            chunkSteps: (chunk: any) => {
                promise.resolve({ value: chunk, done: false });
            },
            closeSteps: () => {
                promise.resolve({ value: undefined, done: true });
            },
            errorSteps: (e: any) => {
                promise.reject(e);
            },
        };

        // 4. Perform ! ReadableStreamDefaultReaderRead(this, readRequest).
        readableStreamDefaultReaderRead(this, readRequest);

        // 5. Return promise.
        return promise.promise;
    }

    releaseLock() {
        //x
        if (!isReadableStreamDefaultReader(this)) {
            throw new TypeError(
                "ReadableStreamDefaultReader.prototype.releaseLock can only be used on a ReadableStreamDefaultReader",
            );
        }

        // 1. If this.[[stream]] is undefined, return.
        if (this[_stream] === undefined) return;

        // 2. Perform ! ReadableStreamDefaultReaderRelease(this).
        readableStreamDefaultReaderRelease(this);
    }
}

// https://streams.spec.whatwg.org/#readablestreambyobreader
// ---------------------------------------------------------------|
//                 ReadableStreamBYOBReader                       |
// ---------------------------------------------------------------|
function isReadableStreamBYOBReader(
    reader: any,
): reader is ReadableStreamBYOBReader {
    return isObject(reader) && !!reader[_readIntoRequests];
}

//x https://streams.spec.whatwg.org/#abstract-opdef-readablestreambyobreadererrorreadintorequests
function readableStreamBYOBReaderErrorReadIntoRequests(
    reader: ReadableStreamBYOBReader,
    e: any,
) {
    // 1. Let readIntoRequests be reader.[[readIntoRequests]].
    const readIntoRequests = reader[_readIntoRequests];
    // 2. Set reader.[[readIntoRequests]] to a new empty list.
    reader[_readIntoRequests] = [];
    // 3. For each readIntoRequest of readIntoRequests, Perform readIntoRequest’s error steps, given e.
    readIntoRequests.forEach((readIntoRequest) =>
        readIntoRequest.errorSteps(e),
    );
}

//x https://streams.spec.whatwg.org/#abstract-opdef-readablestreambyobreaderrelease
function readableStreamBYOBReaderRelease(reader: ReadableStreamBYOBReader) {
    // 1. Perform ! ReadableStreamReaderGenericRelease(reader).
    readableStreamReaderGenericRelease(reader);
    // 2. Let e be a new TypeError exception.
    const e = new TypeError("Reader is released");
    // 3. Perform ! ReadableStreamBYOBReaderErrorReadIntoRequests(reader, e).
    readableStreamBYOBReaderErrorReadIntoRequests(reader, e);
}

//x https://streams.spec.whatwg.org/#readable-stream-add-read-into-request
const readableStreamAddReadIntoRequest = (
    stream: ReadableStream,
    readIntoRequest: ReadIntoRequest,
) => {
    // 1. Assert: stream.[[reader]] implements ReadableStreamBYOBReader.
    if (!isReadableStreamBYOBReader(stream[_reader]))
        throw new TypeError("Stream reader is not a BYOB reader");
    // 2. Assert: stream.[[state]] is "readable" or "closed".
    if (stream[_state] !== "readable" && stream[_state] !== "closed")
        throw new TypeError("Stream is not readable or closed");
    // 3. Append readRequest to stream.[[reader]].[[readIntoRequests]].
    stream[_reader][_readIntoRequests].push(readIntoRequest);
};

//x https://streams.spec.whatwg.org/#readable-byte-stream-controller-fill-head-pull-into-descriptor
const readableByteStreamControllerFillHeadPullIntoDescriptor = (
    controller: ReadableByteStreamController,
    size: number,
    pullIntoDescriptor: PullIntoDescriptor,
) => {
    // 1. Assert: either controller.[[pendingPullIntos]] is empty, or controller.[[pendingPullIntos]][0] is pullIntoDescriptor.
    assert(
        controller[_pendingPullIntos].length === 0 ||
            controller[_pendingPullIntos][0] === pullIntoDescriptor,
        "PullIntoDescriptor is not the head of the queue",
    );
    // 2. Assert: controller.[[byobRequest]] is null.
    assert(controller[_byobRequest] === null, "BYOB request is not null");
    // 3. Set pullIntoDescriptor’s bytes filled to bytes filled + size.
    pullIntoDescriptor.bytesFilled += size;
};

//x https://streams.spec.whatwg.org/#readable-byte-stream-controller-fill-pull-into-descriptor-from-queue
const readableByteStreamControllerFillPullIntoDescriptorFromQueue = (
    controller: ReadableByteStreamController,
    pullIntoDescriptor: PullIntoDescriptor,
) => {
    // 1. Let maxBytesToCopy be min(controller.[[queueTotalSize]], pullIntoDescriptor’s byte length − pullIntoDescriptor’s bytes filled).
    const maxBytesToCopy = Math.min(
        controller[_queueTotalSize],
        pullIntoDescriptor.byteLength - pullIntoDescriptor.bytesFilled,
    );
    // 2. Let maxBytesFilled be pullIntoDescriptor’s bytes filled + maxBytesToCopy.
    const maxBytesFilled = pullIntoDescriptor.bytesFilled + maxBytesToCopy;
    // 3. Let totalBytesToCopyRemaining be maxBytesToCopy.
    let totalBytesToCopyRemaining = maxBytesToCopy;
    // 4. Let ready be false.
    let ready = false;
    // 5. Assert: pullIntoDescriptor’s bytes filled < pullIntoDescriptor’s minimum fill.
    assert(
        pullIntoDescriptor.bytesFilled < pullIntoDescriptor.minimumFill,
        "Bytes filled is greater than minimum fill",
    );
    // 6. Let remainderBytes be the remainder after dividing maxBytesFilled by pullIntoDescriptor’s element size.
    const remainderBytes = maxBytesFilled % pullIntoDescriptor.elementSize;
    // 7. Let maxAlignedBytes be maxBytesFilled − remainderBytes.
    const maxAlignedBytes = maxBytesFilled - remainderBytes;
    // 8. If maxAlignedBytes ≥ pullIntoDescriptor’s minimum fill,
    if (maxAlignedBytes >= pullIntoDescriptor.minimumFill) {
        // 8.1. Set totalBytesToCopyRemaining to maxAlignedBytes − pullIntoDescriptor’s bytes filled.
        totalBytesToCopyRemaining =
            maxAlignedBytes - pullIntoDescriptor.bytesFilled;
        // 8.2. Set ready to true.
        ready = true;
    }
    // 9. Let queue be controller.[[queue]].
    const queue = controller[_queue];
    // 10. While totalBytesToCopyRemaining > 0,
    while (totalBytesToCopyRemaining > 0) {
        // 10.1. Let headOfQueue be queue[0].
        const headOfQueue = queue.peek();
        // 10.2. Let bytesToCopy be min(totalBytesToCopyRemaining, headOfQueue’s byte length).
        const bytesToCopy = Math.min(
            totalBytesToCopyRemaining,
            headOfQueue.byteLength,
        );
        // 10.3 Let destStart be pullIntoDescriptor’s byte offset + pullIntoDescriptor’s bytes filled.
        const destStart =
            pullIntoDescriptor.byteOffset + pullIntoDescriptor.bytesFilled;
        // 10.4 Perform ! CopyDataBlockBytes(pullIntoDescriptor’s buffer.[[ArrayBufferData]], destStart, headOfQueue’s buffer.[[ArrayBufferData]], headOfQueue’s byte offset, bytesToCopy).
        const destBuffer = new Uint8Array(
            pullIntoDescriptor.buffer,
            destStart,
            bytesToCopy,
        );
        const srcBuffer = new Uint8Array(
            headOfQueue.buffer,
            headOfQueue.byteOffset,
            bytesToCopy,
        );
        destBuffer.set(srcBuffer);
        // 10.5. If headOfQueue’s byte length is bytesToCopy,
        if (headOfQueue.byteLength === bytesToCopy) {
            // 10.5.1. Remove queue[0].
            queue.dequeue();
        } else {
            // 10.6. Otherwise,
            // 10.6.1. Set headOfQueue’s byte offset to headOfQueue’s byte offset + bytesToCopy.
            headOfQueue.byteOffset += bytesToCopy;
            // 10.6.2. Set headOfQueue’s byte length to headOfQueue’s byte length − bytesToCopy.
            headOfQueue.byteLength -= bytesToCopy;
        }
        // 10.7. Set controller.[[queueTotalSize]] to controller.[[queueTotalSize]] − bytesToCopy.
        controller[_queueTotalSize] -= bytesToCopy;
        // 10.8. Perform ! ReadableByteStreamControllerFillHeadPullIntoDescriptor(controller, bytesToCopy, pullIntoDescriptor).
        readableByteStreamControllerFillHeadPullIntoDescriptor(
            controller,
            bytesToCopy,
            pullIntoDescriptor,
        );
        // 10.9. Set totalBytesToCopyRemaining to totalBytesToCopyRemaining − bytesToCopy.
        totalBytesToCopyRemaining -= bytesToCopy;
    }
    // 11. If ready is false,
    if (ready === false) {
        // 11.1. Assert: controller.[[queueTotalSize]] is 0.
        assert(controller[_queueTotalSize] === 0, "Queue total size is not 0");
        // 11.2. Assert: pullIntoDescriptor’s bytes filled > 0.
        assert(
            pullIntoDescriptor.bytesFilled > 0,
            "Bytes filled is not greater than 0",
        );
        // 11.3. Assert: pullIntoDescriptor’s bytes filled < pullIntoDescriptor’s minimum fill.
        assert(
            pullIntoDescriptor.bytesFilled < pullIntoDescriptor.minimumFill,
            "Bytes filled is greater than minimum fill",
        );
    }
    // 12. Return ready.
    return ready;
};

//x https://streams.spec.whatwg.org/#readable-byte-stream-controller-convert-pull-into-descriptor
const readableByteStreamControllerConvertPullIntoDescriptor = (
    pullIntoDescriptor: PullIntoDescriptor,
) => {
    // 1. Let bytesFilled be pullIntoDescriptor’s bytes filled.
    const bytesFilled = pullIntoDescriptor.bytesFilled;
    // 2. Let elementSize be pullIntoDescriptor’s element size.
    const elementSize = pullIntoDescriptor.elementSize;
    // 3. Assert: bytesFilled ≤ pullIntoDescriptor’s byte length.
    assert(
        bytesFilled <= pullIntoDescriptor.byteLength,
        "Bytes filled is greater than byte length",
    );
    // 4. Assert: the remainder after dividing bytesFilled by elementSize is 0.
    assert(
        bytesFilled % elementSize === 0,
        "Bytes filled is not divisible by element size",
    );
    // 5. Let buffer be ! TransferArrayBuffer(pullIntoDescriptor’s buffer).
    const buffer = transferArrayBuffer(pullIntoDescriptor.buffer);
    // 6. Return ! Construct(pullIntoDescriptor’s view constructor, « buffer, pullIntoDescriptor’s byte offset, bytesFilled ÷ elementSize »).
    return new pullIntoDescriptor.viewConstructor(
        buffer,
        pullIntoDescriptor.byteOffset,
        bytesFilled / elementSize,
    );
};

//x https://streams.spec.whatwg.org/#readable-byte-stream-controller-clear-algorithms
const readableByteStreamControllerClearAlgorithms = (
    controller: ReadableByteStreamController,
) => {
    // 1. Set controller.[[pullAlgorithm]] to undefined.
    controller[_pullAlgorithm] = undefined;
    // 2. Set controller.[[cancelAlgorithm]] to undefined.
    controller[_cancelAlgorithm] = undefined;
};

//x https://streams.spec.whatwg.org/#readable-stream-has-byob-reader
const readableStreamHasBYOBReader = (stream: ReadableStream) => {
    // 1. Let reader be stream.[[reader]].
    const reader = stream[_reader];
    // 2. If reader is undefined, return false.
    if (reader === undefined) return false;
    // 3. If reader implements ReadableStreamBYOBReader, return true.
    if (isReadableStreamBYOBReader(reader)) return true;
    // 4. Return false.
    return false;
};

//x https://streams.spec.whatwg.org/#readable-stream-get-num-read-into-requests
const readableStreamGetNumReadIntoRequests = (stream: ReadableStream) => {
    // 1. Assert: ! ReadableStreamHasBYOBReader(stream) is true.
    assert(
        readableStreamHasBYOBReader(stream) === true,
        "Stream has no BYOB reader",
    );
    // 2. Return stream.[[reader]].[[readIntoRequests]]'s size.
    return (stream[_reader] as ReadableStreamBYOBReader)[_readIntoRequests]
        .length;
};

//x https://streams.spec.whatwg.org/#readable-byte-stream-controller-get-desired-size
const readableByteStreamControllerGetDesiredSize = (
    controller: ReadableByteStreamController,
) => {
    // 1. Let state be controller.[[stream]].[[state]].
    const state = controller[_stream][_state];
    // 2. If state is "errored", return null.
    if (state === "errored") return null;
    // 3. If state is "closed", return 0.
    if (state === "closed") return 0;
    // 4. Return controller.[[strategyHWM]] − controller.[[queueTotalSize]].
    return controller[_strategyHWM] - controller[_queueTotalSize];
};

//x https://streams.spec.whatwg.org/#readable-byte-stream-controller-should-call-pull
const readableByteStreamControllerShouldCallPull = (
    controller: ReadableByteStreamController,
) => {
    // 1. Let stream be controller.[[stream]].
    const stream = controller[_stream];
    // 2. If stream.[[state]] is not "readable", return false.
    if (stream[_state] !== _readable) return false;
    // 3. If controller.[[closeRequested]] is true, return false.
    if (controller[_closeRequested] === true) return false;
    // 4. If controller.[[started]] is false, return false.
    if (controller[_started] === false) return false;
    // 5. If ! ReadableStreamHasDefaultReader(stream) is true and ! ReadableStreamGetNumReadRequests(stream) > 0, return true.
    if (
        readableStreamHasDefaultReader(stream) &&
        readableStreamGetNumReadRequests(stream) > 0
    )
        return true;
    // 6. If ! ReadableStreamHasBYOBReader(stream) is true and ! ReadableStreamGetNumReadIntoRequests(stream) > 0, return true.
    if (
        readableStreamHasBYOBReader(stream) &&
        readableStreamGetNumReadIntoRequests(stream) > 0
    )
        return true;
    // 7. Let desiredSize be ! ReadableByteStreamControllerGetDesiredSize(controller).
    const desiredSize = readableByteStreamControllerGetDesiredSize(controller);
    // 8. Assert: desiredSize is not null.
    assert(desiredSize !== null, "Desired size is null");
    // 9. If desiredSize > 0, return true.
    if (desiredSize! > 0) return true;
    // 10. Return false.
    return false;
};

//x https://streams.spec.whatwg.org/#readable-byte-stream-controller-invalidate-byob-request
const readableByteStreamControllerInvalidateBYOBRequest = (
    controller: ReadableByteStreamController,
) => {
    // 1. If controller.[[byobRequest]] is null, return.
    if (controller[_byobRequest] === null) return;
    // 2. Set controller.[[byobRequest]].[[controller]] to undefined.
    controller[_byobRequest][_controller] = undefined;
    // 3. Set controller.[[byobRequest]].[[view]] to null.
    controller[_byobRequest][_view] = null;
    // 4. Set controller.[[byobRequest]] to null.
    controller[_byobRequest] = null;
};

//x https://streams.spec.whatwg.org/#readable-byte-stream-controller-clear-pending-pull-intos
const readableByteStreamControllerClearPendingPullIntos = (
    controller: ReadableByteStreamController,
) => {
    // 1. Perform ! ReadableByteStreamControllerInvalidateBYOBRequest(controller).
    readableByteStreamControllerInvalidateBYOBRequest(controller);
    // 2. Set controller.[[pendingPullIntos]] to a new empty list.
    controller[_pendingPullIntos] = [];
};

//x https://streams.spec.whatwg.org/#readable-byte-stream-controller-error
const readableByteStreamControllerError = (
    controller: ReadableByteStreamController,
    e: any,
) => {
    // 1. Let stream be controller.[[stream]].
    const stream = controller[_stream];
    // 2. If stream.[[state]] is not "readable", return.
    if (stream[_state] !== "readable") return;
    // 3. Perform ! ReadableByteStreamControllerClearPendingPullIntos(controller).
    readableByteStreamControllerClearPendingPullIntos(controller);
    // 4. Perform ! ResetQueue(controller).
    resetQueue(controller);
    // 5. Perform ! ReadableByteStreamControllerClearAlgorithms(controller).
    readableByteStreamControllerClearAlgorithms(controller);
    // 6. Perform ! ReadableStreamError(stream, e).
    readableStreamError(stream, e);
};

//x https://streams.spec.whatwg.org/#readable-byte-stream-controller-call-pull-if-needed
const readableByteStreamControllerCallPullIfNeeded = (
    controller: ReadableByteStreamController,
) => {
    // 1. Let shouldPull be ! ReadableByteStreamControllerShouldCallPull(controller).
    const shouldPull = readableByteStreamControllerShouldCallPull(controller);
    // 2. If shouldPull is false, return.
    if (shouldPull === false) return;
    // 3. If controller.[[pulling]] is true,
    if (controller[_pulling] === true) {
        // 3.1. Set controller.[[pullAgain]] to true.
        controller[_pullAgain] = true;
        // 3.2. Return.
        return;
    }
    // 4. Assert: controller.[[pullAgain]] is false.
    assert(controller[_pullAgain] === false, "Pull again is true");
    // 5. Set controller.[[pulling]] to true.
    controller[_pulling] = true;
    // 6. Let pullPromise be the result of performing controller.[[pullAlgorithm]].
    const pullPromise = controller[_pullAlgorithm]!();
    // 7. Upon fulfillment of pullPromise,
    pullPromise.then(
        () => {
            // 7.1. Set controller.[[pulling]] to false.
            controller[_pulling] = false;
            // 7.2. If controller.[[pullAgain]] is true,
            if (controller[_pullAgain] === true) {
                // 7.2.1. Set controller.[[pullAgain]] to false.
                controller[_pullAgain] = false;
                // 7.2.2. Perform ! ReadableByteStreamControllerCallPullIfNeeded(controller).
                readableByteStreamControllerCallPullIfNeeded(controller);
            }
        },
        (e) => {
            // 7.3. Perform ! ReadableByteStreamControllerError(controller, e).
            readableByteStreamControllerError(controller, e);
        },
    );
};

//x https://streams.spec.whatwg.org/#readable-byte-stream-controller-handle-queue-drain
const readableByteStreamControllerHandleQueueDrain = (
    controller: ReadableByteStreamController,
) => {
    // 1. Assert: controller.[[stream]].[[state]] is "readable".
    assert(
        controller[_stream][_state] === "readable",
        "Stream is not readable",
    );
    // 2. If controller.[[queueTotalSize]] is 0 and controller.[[closeRequested]] is true,
    if (
        controller[_queueTotalSize] === 0 &&
        controller[_closeRequested] === true
    ) {
        // 2.1. Perform ! ReadableByteStreamControllerClearAlgorithms(controller).
        readableByteStreamControllerClearAlgorithms(controller);
        // 2.2. Perform ! ReadableStreamClose(controller.[[stream]]).
        readableStreamClose(controller[_stream]);
    } else {
        // 3.1 Perform ! ReadableByteStreamControllerCallPullIfNeeded(controller).
        readableByteStreamControllerCallPullIfNeeded(controller);
    }
};

//x https://streams.spec.whatwg.org/#readable-byte-stream-controller-pull-into
function readableByteStreamControllerPullInto(
    controller: ReadableByteStreamController,
    view: ArrayBufferView,
    min: number,
    readIntoRequest: ReadIntoRequest,
) {
    // 1. Let stream be controller.[[stream]].
    const stream = controller[_stream];
    // 2. Let elementSize be 1.
    let elementSize = 1;
    // 3. Let ctor be %DataView%.
    let ctor: any = DataView;
    // 4. If view has a [[TypedArrayName]] internal slot (i.e., it is not a DataView),
    if (isTypedArray(view)) {
        // 4.1. Set elementSize to the element size specified in the typed array constructors table for view.[[TypedArrayName]].
        elementSize = view.BYTES_PER_ELEMENT;
        // 4.2. Set ctor to the constructor specified in the typed array constructors table for view.[[TypedArrayName]].
        // FIXME: Getting constructor like this is not safe.
        ctor = view.constructor;
    }
    // 5. Let minimumFill be min × elementSize.
    const minimumFill = min * elementSize;
    // 6. Assert: minimumFill ≥ 0 and minimumFill ≤ view.[[ByteLength]].
    assert(
        minimumFill >= 0 && minimumFill <= view.byteLength,
        "Minimum fill is out of bounds",
    );
    // 7. Assert: the remainder after dividing minimumFill by elementSize is 0.
    assert(
        minimumFill % elementSize === 0,
        "Minimum fill is not divisible by element size",
    );
    // 8. Let byteOffset be view.[[ByteOffset]].
    const byteOffset = view.byteOffset;
    // 9. Let byteLength be view.[[ByteLength]].
    const byteLength = view.byteLength;
    // 10. Let bufferResult be TransferArrayBuffer(view.[[ViewedArrayBuffer]]).
    let bufferResult;
    try {
        bufferResult = transferArrayBuffer(view.buffer as ArrayBuffer);
    } catch (e) {
        // 11.1 If bufferResult is an abrupt completion, Perform readIntoRequest’s error steps, given bufferResult.[[Value]].
        readIntoRequest.errorSteps(e);
        // 11.2 Return.
        return;
    }
    // 12. Let buffer be bufferResult.[[Value]].
    const buffer = bufferResult;
    // 13. Let pullIntoDescriptor be a new pull-into descriptor with
    let pullIntoDescriptor: PullIntoDescriptor = {
        buffer,
        bufferByteLength: buffer.byteLength,
        byteOffset,
        byteLength,
        bytesFilled: 0,
        minimumFill,
        elementSize,
        viewConstructor: ctor,
        readerType: "byob",
    };
    // 14. If controller.[[pendingPullIntos]] is not empty,
    if (controller[_pendingPullIntos].length > 0) {
        // 14.1. Append pullIntoDescriptor to controller.[[pendingPullIntos]].
        controller[_pendingPullIntos].push(pullIntoDescriptor);
        // 14.2. Perform ! ReadableStreamAddReadIntoRequest(stream, readIntoRequest).
        readableStreamAddReadIntoRequest(stream, readIntoRequest);
        // 14.3. Return.
        return;
    }

    // 15. If stream.[[state]] is "closed",
    if (stream[_state] === "closed") {
        // 15.1. Let emptyView be ! Construct(ctor, « pullIntoDescriptor’s buffer, pullIntoDescriptor’s byte offset, 0 »).
        const emptyView = new ctor(
            pullIntoDescriptor.buffer,
            pullIntoDescriptor.byteOffset,
            0,
        );
        // 15.2 Perform readIntoRequest’s close steps, given emptyView.
        readIntoRequest.closeSteps(emptyView);
        // 15.3 Return.
        return;
    }

    // 16. If controller.[[queueTotalSize]] > 0,
    if (controller[_queueTotalSize] > 0) {
        // 16.1. If ! ReadableByteStreamControllerFillPullIntoDescriptorFromQueue(controller, pullIntoDescriptor) is true,
        if (
            readableByteStreamControllerFillPullIntoDescriptorFromQueue(
                controller,
                pullIntoDescriptor,
            )
        ) {
            // 16.1.1. Let filledView be ! ReadableByteStreamControllerConvertPullIntoDescriptor(pullIntoDescriptor).
            const filledView =
                readableByteStreamControllerConvertPullIntoDescriptor(
                    pullIntoDescriptor,
                );
            // 16.1.2. Perform ! ReadableByteStreamControllerHandleQueueDrain(controller).
            readableByteStreamControllerHandleQueueDrain(controller);
            // 16.1.3 Perform readIntoRequest’s chunk steps, given filledView.
            readIntoRequest.chunkSteps(filledView);
            // 16.1.4 Return.
            return;
        }
        // 16.2. If controller.[[closeRequested]] is true,
        if (controller[_closeRequested] === true) {
            // 16.2.1. Let e be a TypeError exception.
            const e = new TypeError("Controller close requested");
            // 16.2.2. Perform ! ReadableByteStreamControllerError(controller, e).
            readableByteStreamControllerError(controller, e);
            // 16.2.3. Perform readIntoRequest’s error steps, given e.
            readIntoRequest.errorSteps(e);
            // 16.2.4. Return.
            return;
        }
    }

    // 17. Append pullIntoDescriptor to controller.[[pendingPullIntos]].
    controller[_pendingPullIntos].push(pullIntoDescriptor);
    // 18. Perform ! ReadableStreamAddReadIntoRequest(stream, readIntoRequest).
    readableStreamAddReadIntoRequest(stream, readIntoRequest);
    // 19. Perform ! ReadableByteStreamControllerCallPullIfNeeded(controller).
    readableByteStreamControllerCallPullIfNeeded(controller);
}

//x https://streams.spec.whatwg.org/#readable-stream-byob-reader-read
function readableStreamBYOBReaderRead(
    reader: ReadableStreamBYOBReader,
    view: ArrayBufferView,
    min: number,
    readIntoRequest: ReadIntoRequest,
) {
    // 1. Let stream be reader.[[stream]].
    const stream = reader[_stream];
    // 2. Assert: stream is not undefined.
    if (stream === undefined) throw new TypeError("Stream is undefined");
    // 3. Set stream.[[disturbed]] to true.
    stream[_disturbed] = true;
    // 4. If stream.[[state]] is "errored", perform readIntoRequest’s error steps given stream.[[storedError]].
    if (stream[_state] === "errored") {
        readIntoRequest.errorSteps(stream[_storedError]);
    } else {
        // 5. Otherwise, perform ! ReadableByteStreamControllerPullInto(stream.[[controller]], view, min, readIntoRequest).
        readableByteStreamControllerPullInto(
            stream[_controller] as ReadableByteStreamController,
            view,
            min,
            readIntoRequest,
        );
    }
}

//x https://streams.spec.whatwg.org/#set-up-readable-stream-byob-reader
const setUpReadableStreamBYOBReader = (
    reader: ReadableStreamBYOBReader,
    stream: ReadableStream,
) => {
    // 1. If ! IsReadableStreamLocked(stream) is true, throw a TypeError exception.
    if (isReadableStreamLocked(stream)) throw new TypeError("Stream is locked");
    // 2. If stream.[[controller]] does not implement ReadableByteStreamController, throw a TypeError exception.
    if (
        !isPrototypeOf(
            stream[_controller],
            ReadableByteStreamController.prototype,
        )
    )
        throw new TypeError(
            "Stream controller is not a byte stream controller",
        );
    // 3. Perform ! ReadableStreamReaderGenericInitialize(reader, stream).
    readableStreamReaderGenericInitialize(reader, stream);
    // 3. Set reader.[[readIntoRequests]] to a new empty list.
    reader[_readIntoRequests] = [];
};

//x https://streams.spec.whatwg.org/#byob-reader-class
class ReadableStreamBYOBReader implements IReadableStreamBYOBReader {
    [_closedPromise]: Deferred;
    [_stream]: ReadableStream;
    [_readIntoRequests]: ReadIntoRequest[];

    constructor(stream: ReadableStream) {
        //x
        if (!isReadableStream(stream)) {
            throw new TypeError(
                "ReadableStreamBYOBReader can only be constructed with a ReadableStream instance",
            );
        }

        setUpReadableStreamBYOBReader(this, stream);
    }

    read(
        view: ArrayBufferView,
        options?: ReadableStreamBYOBReaderReadOptions | undefined,
    ): Promise<ReadableStreamReadResult> {
        //x
        if (!isReadableStreamBYOBReader(this)) {
            return Promise.reject(
                new TypeError(
                    "ReadableStreamBYOBReader.prototype.read can only be used on a ReadableStreamBYOBReader",
                ),
            );
        }

        if (!ArrayBuffer.isView(view))
            return Promise.reject(
                new TypeError("View must be an ArrayBufferView"),
            );
        // 1. If view.[[ByteLength]] is 0, return a promise rejected with a TypeError exception.
        if (view.byteLength === 0)
            return Promise.reject(new TypeError("View byteLength is 0"));
        // 2. If view.[[ViewedArrayBuffer]].[[ArrayBufferByteLength]] is 0, return a promise rejected with a TypeError exception.
        if (view.buffer.byteLength === 0)
            return Promise.reject(new TypeError("View buffer byteLength is 0"));
        // 3. If ! IsDetachedBuffer(view.[[ViewedArrayBuffer]]) is true, return a promise rejected with a TypeError exception.
        if (is_array_buffer_detached(view.buffer))
            return Promise.reject(new TypeError("View buffer is detached"));
        // 4. If options["min"] is 0, return a promise rejected with a TypeError exception.
        if (options && options.min === 0)
            return Promise.reject(new TypeError("Options min is 0"));
        // 5 If view has a [[TypedArrayName]] internal slot,
        // 5.1 If options["min"] > view.[[ArrayLength]], return a promise rejected with a RangeError exception.
        // 6. Otherwise (i.e., it is a DataView),
        // 6.1 If options["min"] > view.[[ByteLength]], return a promise rejected with a RangeError exception.
        // FIXME: check the type of view
        if (options?.min && options.min > view.byteLength)
            return Promise.reject(
                new RangeError("Options min is greater than view byteLength"),
            );
        // 7. If this.[[stream]] is undefined, return a promise rejected with a TypeError exception.
        if (this[_stream] === undefined)
            return Promise.reject(
                new TypeError("Reader is not associated with a stream"),
            );
        // 8. Let promise be a new promise.
        const promise = new Deferred<ReadableStreamReadResult>();
        // 9. Let readIntoRequest be a new read-into request with the following items:
        const readIntoRequest = {
            chunkSteps: (chunk: ArrayBufferView) => {
                promise.resolve({ value: chunk, done: false });
            },
            closeSteps: (chunk?: ArrayBufferView) => {
                promise.resolve({ value: chunk, done: true });
            },
            errorSteps: (e: any) => {
                promise.reject(e);
            },
        };
        // 10. Perform ! ReadableStreamBYOBReaderRead(this, view, options["min"], readIntoRequest).
        readableStreamBYOBReaderRead(
            this,
            view,
            options?.min || 1,
            readIntoRequest,
        );
        // 11. Return promise.
        return promise.promise;
    }

    cancel(reason: any): Promise<void> {
        //x
        if (!isReadableStreamBYOBReader(this)) {
            return Promise.reject(
                new TypeError(
                    "ReadableStreamBYOBReader.prototype.cancel can only be used on a ReadableStreamBYOBReader",
                ),
            );
        }

        // 1. If this.[[stream]] is undefined, return a promise rejected with a TypeError exception.
        if (this[_stream] === undefined)
            return Promise.reject(
                new TypeError("Reader is not associated with a stream"),
            );

        // 2. Return ! ReadableStreamReaderGenericCancel(this, reason).
        return readableStreamReaderGenericCancel(this, reason);
    }

    get closed() {
        //x
        if (!isReadableStreamBYOBReader(this)) {
            return Promise.reject(
                new TypeError(
                    "ReadableStreamBYOBReader.prototype.closed can only be used on a ReadableStreamBYOBReader",
                ),
            );
        }

        return this[_closedPromise].promise;
    }

    releaseLock(): void {
        //x
        if (!isReadableStreamBYOBReader(this)) {
            throw new TypeError(
                "ReadableStreamBYOBReader.prototype.releaseLock can only be used on a ReadableStreamBYOBReader",
            );
        }

        // 1. If this.[[stream]] is undefined, return.
        if (this[_stream] === undefined) return;

        // 2. Perform ! ReadableStreamBYOBReaderRelease(this).
        readableStreamBYOBReaderRelease(this);
    }
}

// https://streams.spec.whatwg.org/#readablestreambyobrequest
// ---------------------------------------------------------------|
//                ReadableByteStreamController                    |
// ---------------------------------------------------------------|

// TODO: each call to this function should check if the buffer is detached.
//x https://streams.spec.whatwg.org/#transfer-array-buffer
function transferArrayBuffer(buffer: ArrayBuffer): ArrayBuffer {
    // 1. Assert: ! IsDetachedBuffer(O) is false.
    // assert(is_array_buffer_detached(buffer) === false, "Buffer is detached");
    // https://tc39.es/proposal-arraybuffer-transfer/#sec-arraybuffer.prototype.transfer
    return buffer.transfer();
}

//FIXME: https://streams.spec.whatwg.org/#can-transfer-array-buffer
const canTransferArrayBuffer = (buffer: ArrayBuffer) => {
    // 1. Assert: Type(O) is Object.
    assert(typeof buffer === "object", "Buffer is not an object");
    // 2. Assert: O has an [[ArrayBufferData]] internal slot.
    // FIXME: Check for any array buffer internal slot. e.g: SharedArrayBuffer, ArrayBuffer.
    assert(isArrayBuffer(buffer), "Buffer is not an ArrayBuffer");
    // 3. If ! IsDetachedBuffer(O) is true, return false.
    if (is_array_buffer_detached(buffer)) return false;
    // 4. If SameValue(O.[[ArrayBufferDetachKey]], undefined) is false, return false.
    // FIXME: Check for detach key.
    // 5. Return true.
    return true;
};

//x https://streams.spec.whatwg.org/#readable-byte-stream-controller-shift-pending-pull-into
const readableByteStreamControllerShiftPendingPullInto = (
    controller: ReadableByteStreamController,
) => {
    // 1. Assert: controller.[[byobRequest]] is null.
    assert(controller[_byobRequest] === null, "BYOB request is not null");
    // 2. Let descriptor be controller.[[pendingPullIntos]][0].
    const descriptor = controller[_pendingPullIntos][0];
    // 3. Remove descriptor from controller.[[pendingPullIntos]].
    controller[_pendingPullIntos].shift();
    // 4. Return descriptor.
    return descriptor;
};

//x https://streams.spec.whatwg.org/#readable-stream-fulfill-read-into-request
const readableStreamFulfillReadIntoRequest = (
    stream: ReadableStream,
    chunk: ArrayBufferView,
    done: boolean,
) => {
    // 1. Assert: ! ReadableStreamHasBYOBReader(stream) is true.
    assert(readableStreamHasBYOBReader(stream), "Stream has no BYOB reader");
    // 2. Let reader be stream.[[reader]].
    const reader = stream[_reader] as ReadableStreamBYOBReader;
    // 3. Assert: reader.[[readIntoRequests]] is not empty.
    assert(reader[_readIntoRequests].length > 0, "Read into requests is empty");
    // 4. Let readIntoRequest be reader.[[readIntoRequests]][0].
    const readIntoRequest = reader[_readIntoRequests][0];
    // 5. Remove readIntoRequest from reader.[[readIntoRequests]].
    reader[_readIntoRequests].shift();
    // 6. If done is true, perform readIntoRequest’s close steps, given chunk.
    if (done) readIntoRequest.closeSteps(chunk);
    // 7. Otherwise, perform readIntoRequest’s chunk steps, given chunk.
    else readIntoRequest.chunkSteps(chunk);
};

//x https://streams.spec.whatwg.org/#readable-byte-stream-controller-commit-pull-into-descriptor
const readableByteStreamControllerCommitPullIntoDescriptor = (
    stream: ReadableStream,
    pullIntoDescriptor: PullIntoDescriptor,
) => {
    // 1. Assert: stream.[[state]] is not "errored".
    assert(stream[_state] !== "errored", "Stream is errored");
    // 2. Assert: pullIntoDescriptor.reader type is not "none".
    assert(pullIntoDescriptor.readerType !== "none", "Reader type is none");
    // 3. Let done be false.
    let done = false;
    // 4. If stream.[[state]] is "closed",
    if (stream[_state] === "closed") {
        // 4.1 Assert: the remainder after dividing pullIntoDescriptor’s bytes filled by pullIntoDescriptor’s element size is 0.
        assert(
            pullIntoDescriptor.bytesFilled % pullIntoDescriptor.elementSize ===
                0,
            "Bytes filled is not divisible by element size",
        );
        // 4.2 Set done to true.
        done = true;
    }
    // 5. Let filledView be ! ReadableByteStreamControllerConvertPullIntoDescriptor(pullIntoDescriptor).
    const filledView =
        readableByteStreamControllerConvertPullIntoDescriptor(
            pullIntoDescriptor,
        );
    // 6. If pullIntoDescriptor’s reader type is "default",
    if (pullIntoDescriptor.readerType === "default") {
        // 6.1 Perform ! ReadableStreamFulfillReadRequest(stream, filledView, done).
        readableStreamFulfillReadRequest(stream, filledView, done);
    } else {
        // 7.1 Otherwise, Assert: pullIntoDescriptor’s reader type is "byob".
        assert(
            pullIntoDescriptor.readerType === "byob",
            "Reader type is not byob",
        );
        // 7.2 Perform ! ReadableStreamFulfillReadIntoRequest(stream, filledView, done).
        readableStreamFulfillReadIntoRequest(stream, filledView, done);
    }
};

//x https://streams.spec.whatwg.org/#readable-byte-stream-controller-respond-in-closed-state
const readableByteStreamControllerRespondInClosedState = (
    controller: ReadableByteStreamController,
    firstDescriptor: PullIntoDescriptor,
) => {
    // 1. Assert: the remainder after dividing firstDescriptor’s bytes filled by firstDescriptor’s element size is 0.
    assert(
        firstDescriptor.bytesFilled % firstDescriptor.elementSize === 0,
        "Bytes filled is not divisible by element size",
    );
    // 2. If firstDescriptor’s reader type is "none", perform ! ReadableByteStreamControllerShiftPendingPullInto(controller).
    if (firstDescriptor.readerType === "none")
        readableByteStreamControllerShiftPendingPullInto(controller);
    // 3. Let stream be controller.[[stream]].
    const stream = controller[_stream];
    // 4. If ! ReadableStreamHasBYOBReader(stream) is true,
    if (readableStreamHasBYOBReader(stream)) {
        // 4.1 While ! ReadableStreamGetNumReadIntoRequests(stream) > 0,
        while (readableStreamGetNumReadIntoRequests(stream) > 0) {
            // 4.1.1 Let pullIntoDescriptor be ! ReadableByteStreamControllerShiftPendingPullInto(controller).
            const pullIntoDescriptor =
                readableByteStreamControllerShiftPendingPullInto(controller);
            // 4.1.2 Perform ! ReadableByteStreamControllerCommitPullIntoDescriptor(stream, pullIntoDescriptor).
            readableByteStreamControllerCommitPullIntoDescriptor(
                stream,
                pullIntoDescriptor,
            );
        }
    }
};

//x https://streams.spec.whatwg.org/#readable-byte-stream-controller-enqueue-chunk-to-queue
const readableByteStreamControllerEnqueueChunkToQueue = (
    controller: ReadableByteStreamController,
    buffer: ArrayBuffer,
    byteOffset: number,
    byteLength: number,
) => {
    // 1. Append a new readable byte stream queue entry with buffer buffer, byte offset byteOffset, and byte length byteLength to controller.[[queue]].
    controller[_queue].enqueue({ buffer, byteOffset, byteLength });
    // 2. Set controller.[[queueTotalSize]] to controller.[[queueTotalSize]] + byteLength.
    controller[_queueTotalSize] += byteLength;
};

//x https://streams.spec.whatwg.org/#abstract-opdef-readablebytestreamcontrollerenqueueclonedchunktoqueue
const readableByteStreamControllerEnqueueClonedChunkToQueue = (
    controller: ReadableByteStreamController,
    buffer: ArrayBuffer,
    byteOffset: number,
    byteLength: number,
) => {
    let cloneResult;
    try {
        // 1. Let cloneResult be CloneArrayBuffer(buffer, byteOffset, byteLength, %ArrayBuffer%).
        cloneResult = buffer.slice(byteOffset, byteOffset + byteLength);
    } catch (error) {
        // 2. If cloneResult is an abrupt completion,
        // 2.1 Perform ! ReadableByteStreamControllerError(controller, cloneResult.[[Value]]).
        readableByteStreamControllerError(controller, error);
        // 2.2 Return cloneResult.
        return cloneResult;
    }
    // 3. Perform ! ReadableByteStreamControllerEnqueueChunkToQueue(controller, cloneResult.[[Value]], 0, byteLength).
    readableByteStreamControllerEnqueueChunkToQueue(
        controller,
        cloneResult,
        0,
        byteLength,
    );
};

//x https://streams.spec.whatwg.org/#abstract-opdef-readablebytestreamcontrollerenqueuedetachedpullintotoqueue
const readableByteStreamControllerEnqueueDetachedPullIntoToQueue = (
    controller: ReadableByteStreamController,
    pullIntoDescriptor: PullIntoDescriptor,
) => {
    // 1. Assert: pullIntoDescriptor’s reader type is "none".
    assert(pullIntoDescriptor.readerType === "none", "Reader type is not none");
    // 2. If pullIntoDescriptor’s bytes filled > 0, perform ? ReadableByteStreamControllerEnqueueClonedChunkToQueue(controller, pullIntoDescriptor’s buffer, pullIntoDescriptor’s byte offset, pullIntoDescriptor’s bytes filled).
    if (pullIntoDescriptor.bytesFilled > 0)
        readableByteStreamControllerEnqueueClonedChunkToQueue(
            controller,
            pullIntoDescriptor.buffer,
            pullIntoDescriptor.byteOffset,
            pullIntoDescriptor.bytesFilled,
        );
    // 3. Perform ! ReadableByteStreamControllerShiftPendingPullInto(controller).
    readableByteStreamControllerShiftPendingPullInto(controller);
};

//x https://streams.spec.whatwg.org/#readable-byte-stream-controller-process-pull-into-descriptors-using-queue
const readableByteStreamControllerProcessPullIntoDescriptorsUsingQueue = (
    controller: ReadableByteStreamController,
) => {
    // 1. Assert: controller.[[closeRequested]] is false.
    assert(controller[_closeRequested] === false, "Close requested is true");
    // 2. While controller.[[pendingPullIntos]] is not empty,
    while (controller[_pendingPullIntos].length > 0) {
        // 2.1 If controller.[[queueTotalSize]] is 0, return.
        if (controller[_queueTotalSize] === 0) return;
        // 2.2 Let pullIntoDescriptor be controller.[[pendingPullIntos]][0].
        const pullIntoDescriptor = controller[_pendingPullIntos][0];
        // 2.3 If ! ReadableByteStreamControllerFillPullIntoDescriptorFromQueue(controller, pullIntoDescriptor) is true,
        if (
            readableByteStreamControllerFillPullIntoDescriptorFromQueue(
                controller,
                pullIntoDescriptor,
            )
        ) {
            // 2.3.1 Perform ! ReadableByteStreamControllerShiftPendingPullInto(controller).
            readableByteStreamControllerShiftPendingPullInto(controller);
            // 2.3.2 Perform ! ReadableByteStreamControllerCommitPullIntoDescriptor(controller.[[stream]], pullIntoDescriptor).
            readableByteStreamControllerCommitPullIntoDescriptor(
                controller[_stream],
                pullIntoDescriptor,
            );
        }
    }
};

//x https://streams.spec.whatwg.org/#readable-byte-stream-controller-respond-in-readable-state
const readableByteStreamControllerRespondInReadableState = (
    controller: ReadableByteStreamController,
    bytesWritten: number,
    firstDescriptor: PullIntoDescriptor,
) => {
    // 1. Assert: pullIntoDescriptor’s bytes filled + bytesWritten ≤ pullIntoDescriptor’s byte length.
    assert(
        firstDescriptor.bytesFilled + bytesWritten <=
            firstDescriptor.byteLength,
        "Bytes filled + bytes written is greater than byte length",
    );
    // 2. Perform ! ReadableByteStreamControllerFillHeadPullIntoDescriptor(controller, bytesWritten, pullIntoDescriptor).
    readableByteStreamControllerFillHeadPullIntoDescriptor(
        controller,
        bytesWritten,
        firstDescriptor,
    );
    // 3. If pullIntoDescriptor’s reader type is "none",
    if (firstDescriptor.readerType === "none") {
        // 3.1 Perform ? ReadableByteStreamControllerEnqueueDetachedPullIntoToQueue(controller, pullIntoDescriptor).
        readableByteStreamControllerEnqueueDetachedPullIntoToQueue(
            controller,
            firstDescriptor,
        );
        // 3.2 Perform ! ReadableByteStreamControllerProcessPullIntoDescriptorsUsingQueue(controller).
        readableByteStreamControllerProcessPullIntoDescriptorsUsingQueue(
            controller,
        );
        // 3.3 Return.
        return;
    }
    // 4. If pullIntoDescriptor’s bytes filled < pullIntoDescriptor’s minimum fill, return.
    if (firstDescriptor.bytesFilled < firstDescriptor.minimumFill) return;
    // 5. Perform ! ReadableByteStreamControllerShiftPendingPullInto(controller).
    readableByteStreamControllerShiftPendingPullInto(controller);
    // 6. Let remainderSize be the remainder after dividing pullIntoDescriptor’s bytes filled by pullIntoDescriptor’s element size.
    const remainderSize =
        firstDescriptor.bytesFilled % firstDescriptor.elementSize;
    // 7. If remainderSize > 0,
    if (remainderSize > 0) {
        // 7.1 Let end be pullIntoDescriptor’s byte offset + pullIntoDescriptor’s bytes filled.
        const end = firstDescriptor.byteOffset + firstDescriptor.bytesFilled;
        // 7.2 Perform ? ReadableByteStreamControllerEnqueueClonedChunkToQueue(controller, pullIntoDescriptor’s buffer, end − remainderSize, remainderSize).
        readableByteStreamControllerEnqueueClonedChunkToQueue(
            controller,
            firstDescriptor.buffer,
            end - remainderSize,
            remainderSize,
        );
    }
    // 8. Set pullIntoDescriptor’s bytes filled to pullIntoDescriptor’s bytes filled − remainderSize.
    firstDescriptor.bytesFilled -= remainderSize;
    // 9. Perform ! ReadableByteStreamControllerCommitPullIntoDescriptor(controller.[[stream]], pullIntoDescriptor).
    readableByteStreamControllerCommitPullIntoDescriptor(
        controller[_stream],
        firstDescriptor,
    );
    // 10. Perform ! ReadableByteStreamControllerProcessPullIntoDescriptorsUsingQueue(controller).
    readableByteStreamControllerProcessPullIntoDescriptorsUsingQueue(
        controller,
    );
};

//x https://streams.spec.whatwg.org/#readable-byte-stream-controller-respond-internal
const readableByteStreamControllerRespondInternal = (
    controller: ReadableByteStreamController,
    bytesWritten: number,
) => {
    // 1. Let firstDescriptor be controller.[[pendingPullIntos]][0].
    const firstDescriptor = controller[_pendingPullIntos][0];
    // 2. Assert: ! CanTransferArrayBuffer(firstDescriptor’s buffer) is true.
    assert(
        canTransferArrayBuffer(firstDescriptor.buffer),
        "Cannot transfer array buffer",
    );
    // 3. Perform ! ReadableByteStreamControllerInvalidateBYOBRequest(controller).
    readableByteStreamControllerInvalidateBYOBRequest(controller);
    // 4. Let state be controller.[[stream]].[[state]].
    const state = controller[_stream][_state];
    // 5. If state is "closed",
    if (state === "closed") {
        // 5.1. Assert: bytesWritten is 0.
        assert(bytesWritten === 0, "Bytes written is not 0");
        // 5.2. Perform ! ReadableByteStreamControllerRespondInClosedState(controller, firstDescriptor).
        readableByteStreamControllerRespondInClosedState(
            controller,
            firstDescriptor,
        );
    } else {
        // 6.1 Assert: state is "readable".
        assert(state === "readable", "Stream is not readable");
        // 6.2 Assert: bytesWritten > 0.
        assert(bytesWritten > 0, "Bytes written is not greater than 0");
        // 6.3 Perform ? ReadableByteStreamControllerRespondInReadableState(controller, bytesWritten, firstDescriptor).
        readableByteStreamControllerRespondInReadableState(
            controller,
            bytesWritten,
            firstDescriptor,
        );
    }
    // 7. Perform ! ReadableByteStreamControllerCallPullIfNeeded(controller).
    readableByteStreamControllerCallPullIfNeeded(controller);
};

//x https://streams.spec.whatwg.org/#readable-byte-stream-controller-respond
function readableByteStreamControllerRespond(
    controller: ReadableByteStreamController,
    bytesWritten: number,
) {
    // 1. Assert: controller.[[pendingPullIntos]] is not empty.
    assert(
        controller[_pendingPullIntos].length > 0,
        "Pending pull intos is empty",
    );
    // 2. Let firstDescriptor be controller.[[pendingPullIntos]][0].
    const firstDescriptor = controller[_pendingPullIntos][0];
    // 3. Let state be controller.[[stream]].[[state]].
    const state = controller[_stream][_state];
    // 4. If state is "closed", If bytesWritten is not 0, throw a TypeError exception.
    if (state === "closed" && bytesWritten !== 0) {
        throw new TypeError("Bytes written is not 0");
    } else {
        // 5.1 Assert: state is "readable".
        assert(state === "readable", "Stream is not readable");
        // 5.2 If bytesWritten is 0, throw a TypeError exception.
        if (bytesWritten === 0) throw new TypeError("Bytes written is 0");
        // 5.3 If firstDescriptor’s bytes filled + bytesWritten > firstDescriptor’s byte length, throw a RangeError exception.
        if (
            firstDescriptor.bytesFilled + bytesWritten >
            firstDescriptor.byteLength
        )
            throw new RangeError(
                "Bytes filled + bytes written is greater than byte length",
            );
    }

    // 6. Set firstDescriptor’s buffer to ! TransferArrayBuffer(firstDescriptor’s buffer).
    firstDescriptor.buffer = transferArrayBuffer(firstDescriptor.buffer);
    // 7. Perform ? ReadableByteStreamControllerRespondInternal(controller, bytesWritten).
    readableByteStreamControllerRespondInternal(controller, bytesWritten);
}

//x https://streams.spec.whatwg.org/#readable-byte-stream-controller-respond-with-new-view
const readableByteStreamControllerRespondWithNewView = (
    controller: ReadableByteStreamController,
    view: ArrayBufferView,
) => {
    // 1. Assert: controller.[[pendingPullIntos]] is not empty.
    assert(
        controller[_pendingPullIntos].length > 0,
        "Pending pull intos is empty",
    );
    // 2. Assert: ! IsDetachedBuffer(view.[[ViewedArrayBuffer]]) is false.
    assert(
        is_array_buffer_detached(view.buffer) === false,
        "View buffer is detached",
    );
    // 3. Let firstDescriptor be controller.[[pendingPullIntos]][0].
    const firstDescriptor = controller[_pendingPullIntos][0];
    // 4. Let state be controller.[[stream]].[[state]].
    const state = controller[_stream][_state];
    // 5. If state is "closed",
    if (state === "closed") {
        // 5.1. If view.[[ByteLength]] is not 0, throw a TypeError exception.
        if (view.byteLength !== 0)
            throw new TypeError("View byteLength is not 0");
    } else {
        // 6.1 Assert: state is "readable".
        assert(state === "readable", "Stream is not readable");
        // 6.2 If view.[[ByteLength]] is 0, throw a TypeError exception.
        if (view.byteLength === 0) throw new TypeError("View byteLength is 0");
    }
    // 7. If firstDescriptor’s byte offset + firstDescriptor’ bytes filled is not view.[[ByteOffset]], throw a RangeError exception.
    if (
        firstDescriptor.byteOffset + firstDescriptor.bytesFilled !==
        view.byteOffset
    )
        throw new RangeError(
            "Byte offset + bytes filled is not equal to view byte offset",
        );
    // 8. If firstDescriptor’s buffer byte length is not view.[[ViewedArrayBuffer]].[[ByteLength]], throw a RangeError exception.
    if (firstDescriptor.buffer.byteLength !== view.buffer.byteLength)
        throw new RangeError(
            "Buffer byte length is not equal to view buffer byte length",
        );
    // 9. If firstDescriptor’s bytes filled + view.[[ByteLength]] > firstDescriptor’s byte length, throw a RangeError exception.
    if (
        firstDescriptor.bytesFilled + view.byteLength >
        firstDescriptor.byteLength
    )
        throw new RangeError(
            "Bytes filled + view byte length is greater than byte length",
        );
    // 10. Let viewByteLength be view.[[ByteLength]].
    const viewByteLength = view.byteLength;
    // 11. Set firstDescriptor’s buffer to ? TransferArrayBuffer(view.[[ViewedArrayBuffer]]).
    firstDescriptor.buffer = transferArrayBuffer(view.buffer as ArrayBuffer);
    // 12. Perform ? ReadableByteStreamControllerRespondInternal(controller, viewByteLength).
    readableByteStreamControllerRespondInternal(controller, viewByteLength);
};

//x https://streams.spec.whatwg.org/#readablestreambyobrequest
class ReadableStreamBYOBRequest {
    [_view]: ArrayBufferView | null;
    [_controller]?: ReadableByteStreamController;

    get view() {
        //x
        return this[_view];
    }

    respond(bytesWritten: number) {
        //x
        if (!isPrototypeOf(this, ReadableStreamBYOBRequest.prototype)) {
            throw new TypeError(
                "ReadableStreamBYOBRequest.prototype.respond can only be used on a ReadableStreamBYOBRequest",
            );
        }

        // 1. If this.[[controller]] is undefined, throw a TypeError exception.
        if (this[_controller] === undefined)
            throw new TypeError("Controller is undefined");
        // 2. If ! IsDetachedBuffer(this.[[view]].[[ArrayBuffer]]) is true, throw a TypeError exception.
        if (this[_view] && is_array_buffer_detached(this[_view].buffer))
            throw new TypeError("View buffer is detached");
        // 3. Assert: this.[[view]].[[ByteLength]] > 0.
        assert(this[_view]!.byteLength > 0, "View byteLength is 0");
        // 4. Assert: this.[[view]].[[ViewedArrayBuffer]].[[ByteLength]] > 0.
        assert(
            this[_view]!.buffer.byteLength > 0,
            "View buffer byteLength is 0",
        );
        // 5. Perform ? ReadableByteStreamControllerRespond(this.[[controller]], bytesWritten).
        readableByteStreamControllerRespond(this[_controller], bytesWritten);
    }

    respondWithNewView(view: ArrayBufferView) {
        //x
        if (!isPrototypeOf(this, ReadableStreamBYOBRequest.prototype)) {
            throw new TypeError(
                "ReadableStreamBYOBRequest.prototype.respondWithNewView can only be used on a ReadableStreamBYOBRequest",
            );
        }

        // 1. If this.[[controller]] is undefined, throw a TypeError exception.
        if (this[_controller] === undefined)
            throw new TypeError("Controller is undefined");
        // 2. If ! IsDetachedBuffer(view.[[ViewedArrayBuffer]]) is true, throw a TypeError exception.
        if (is_array_buffer_detached(view.buffer))
            throw new TypeError("View buffer is detached");
        // 3. Return ? ReadableByteStreamControllerRespondWithNewView(this.[[controller]], view).
        return readableByteStreamControllerRespondWithNewView(
            this[_controller],
            view,
        );
    }
}

// https://streams.spec.whatwg.org/#readablebytestreamcontroller
// ---------------------------------------------------------------|
//                 ReadableByteStreamController                   |
// ---------------------------------------------------------------|

//x https://streams.spec.whatwg.org/#abstract-opdef-readablebytestreamcontrollergetbyobrequest
const readableByteStreamControllerGetBYOBRequest = (
    controller: ReadableByteStreamController,
) => {
    // 1. If controller.[[byobRequest]] is null and controller.[[pendingPullIntos]] is not empty,
    if (
        controller[_byobRequest] === null &&
        controller[_pendingPullIntos].length > 0
    ) {
        // 1.1. Let firstDescriptor be controller.[[pendingPullIntos]][0].
        const firstDescriptor = controller[_pendingPullIntos][0];
        // 1.2. Let view be ! Construct(%Uint8Array%, « firstDescriptor’s buffer, firstDescriptor’s byte offset + firstDescriptor’s bytes filled, firstDescriptor’s byte length − firstDescriptor’s bytes filled »).
        const view = new Uint8Array(
            firstDescriptor.buffer,
            firstDescriptor.byteOffset + firstDescriptor.bytesFilled,
            firstDescriptor.byteLength - firstDescriptor.bytesFilled,
        );
        // 1.3. Let byobRequest be a new ReadableStreamBYOBRequest.
        const byobRequest = new ReadableStreamBYOBRequest();
        // 1.4. Set byobRequest.[[controller]] to controller.
        byobRequest[_controller] = controller;
        // 1.5. Set byobRequest.[[view]] to view.
        byobRequest[_view] = view;
        // 1.6. Set controller.[[byobRequest]] to byobRequest.
        controller[_byobRequest] = byobRequest;
    }
    // 2. Return controller.[[byobRequest]].
    return controller[_byobRequest];
};

//x https://streams.spec.whatwg.org/#readable-byte-stream-controller-close
const readableByteStreamControllerClose = (
    controller: ReadableByteStreamController,
) => {
    // 1. Let stream be controller.[[stream]].
    const stream = controller[_stream];
    // 2. If controller.[[closeRequested]] is true or stream.[[state]] is not "readable", return.
    if (controller[_closeRequested] === true || stream[_state] !== "readable")
        return;
    // 3. If controller.[[queueTotalSize]] > 0,
    if (controller[_queueTotalSize] > 0) {
        // 3.1. Set controller.[[closeRequested]] to true.
        controller[_closeRequested] = true;
        // 3.2. Return.
        return;
    }
    // 4. If controller.[[pendingPullIntos]] is not empty,
    if (controller[_pendingPullIntos].length > 0) {
        // 4.1. Let firstPendingPullInto be controller.[[pendingPullIntos]][0].
        const firstPendingPullInto = controller[_pendingPullIntos][0];
        // 4.2. If the remainder after dividing firstPendingPullInto’s bytes filled by firstPendingPullInto’s element size is not 0,
        if (
            firstPendingPullInto.bytesFilled %
                firstPendingPullInto.elementSize !==
            0
        ) {
            // 4.2.1. Let e be a new TypeError exception.
            const e = new TypeError(
                "Bytes filled is not divisible by element size",
            );
            // 4.2.2. Perform ! ReadableByteStreamControllerError(controller, e).
            readableByteStreamControllerError(controller, e);
            // 4.2.3. Throw e.
            throw e;
        }
    }
    // 5. Perform ! ReadableByteStreamControllerClearAlgorithms(controller).
    readableByteStreamControllerClearAlgorithms(controller);
    // 6. Perform ! ReadableStreamClose(stream).
    readableStreamClose(stream);
};

//x https://streams.spec.whatwg.org/#abstract-opdef-readablebytestreamcontrollerfillreadrequestfromqueue
const readableByteStreamControllerFillReadRequestFromQueue = (
    controller: ReadableByteStreamController,
    readRequest: ReadRequest,
) => {
    // 1. Assert: controller.[[queueTotalSize]] > 0.
    assert(controller[_queueTotalSize] > 0, "Queue total size is 0");
    // 2. Let entry be controller.[[queue]][0].
    // 3. Remove entry from controller.[[queue]].
    const entry = controller[_queue].dequeue();
    // 4. Set controller.[[queueTotalSize]] to controller.[[queueTotalSize]] − entry’s byte length.
    controller[_queueTotalSize] -= entry.byteLength;
    // 5. Perform ! ReadableByteStreamControllerHandleQueueDrain(controller).
    readableByteStreamControllerHandleQueueDrain(controller);
    // 6. Let view be ! Construct(%Uint8Array%, « entry’s buffer, entry’s byte offset, entry’s byte length »).
    const view = new Uint8Array(
        entry.buffer,
        entry.byteOffset,
        entry.byteLength,
    );
    // 7. Perform readRequest’s chunk steps, given view.
    readRequest.chunkSteps(view);
};

//x https://streams.spec.whatwg.org/#abstract-opdef-readablebytestreamcontrollerprocessreadrequestsusingqueue
const readableByteStreamControllerProcessReadRequestsUsingQueue = (
    controller: ReadableByteStreamController,
) => {
    // 1. Let reader be controller.[[stream]].[[reader]].
    const reader = controller[_stream][_reader] as ReadableStreamDefaultReader;
    // 2. Assert: reader implements ReadableStreamDefaultReader.
    assert(
        isReadableStreamDefaultReader(reader),
        "Reader is not a default reader",
    );
    // 3. While reader.[[readRequests]] is not empty,
    while (reader[_readRequests].length > 0) {
        // 3.1. If controller.[[queueTotalSize]] is 0, return.
        if (controller[_queueTotalSize] === 0) return;
        // 3.2. Let readRequest be reader.[[readRequests]][0].
        const readRequest = reader[_readRequests][0];
        // 3.3. Remove readRequest from reader.[[readRequests]].
        reader[_readRequests].shift();
        // 3.4 Perform ! ReadableByteStreamControllerFillReadRequestFromQueue(controller, readRequest).
        readableByteStreamControllerFillReadRequestFromQueue(
            controller,
            readRequest,
        );
    }
};

//x https://streams.spec.whatwg.org/#readable-byte-stream-controller-enqueue
const readableByteStreamControllerEnqueue = (
    controller: ReadableByteStreamController,
    chunk: ArrayBufferView,
) => {
    // 1. Let stream be controller.[[stream]].
    const stream = controller[_stream];
    // 2. If controller.[[closeRequested]] is true or stream.[[state]] is not "readable", return.
    if (controller[_closeRequested] === true || stream[_state] !== _readable)
        return;
    // 3. Let buffer be chunk.[[ViewedArrayBuffer]].
    const buffer = chunk.buffer as ArrayBuffer;
    // 4. Let byteOffset be chunk.[[ByteOffset]].
    const byteOffset = chunk.byteOffset;
    // 5. Let byteLength be chunk.[[ByteLength]].
    const byteLength = chunk.byteLength;
    // 6. If ! IsDetachedBuffer(buffer) is true, throw a TypeError exception.
    if (buffer.detached) throw new TypeError("Buffer is detached");
    // 7. Let transferredBuffer be ? TransferArrayBuffer(buffer).
    const transferredBuffer = transferArrayBuffer(buffer);
    // 8. If controller.[[pendingPullIntos]] is not empty,
    if (controller[_pendingPullIntos].length > 0) {
        // 8.1. Let firstPendingPullInto be controller.[[pendingPullIntos]][0].
        const firstPendingPullInto = controller[_pendingPullIntos][0];
        // 8.2 If ! IsDetachedBuffer(firstPendingPullInto’s buffer) is true, throw a TypeError exception.
        if (firstPendingPullInto.buffer.detached)
            throw new TypeError("Buffer is detached");
        // 8.3 Perform ! ReadableByteStreamControllerInvalidateBYOBRequest(controller).
        readableByteStreamControllerInvalidateBYOBRequest(controller);
        // 8.4 Set firstPendingPullInto’s buffer to ! TransferArrayBuffer(firstPendingPullInto’s buffer).
        firstPendingPullInto.buffer = transferArrayBuffer(
            firstPendingPullInto.buffer,
        );
        // 8.5 If firstPendingPullInto’s reader type is "none", perform ? ReadableByteStreamControllerEnqueueDetachedPullIntoToQueue(controller, firstPendingPullInto).
        if (firstPendingPullInto.readerType === "none")
            readableByteStreamControllerEnqueueDetachedPullIntoToQueue(
                controller,
                firstPendingPullInto,
            );
    }
    // 9. If ! ReadableStreamHasDefaultReader(stream) is true,
    if (readableStreamHasDefaultReader(stream)) {
        // 9.1. Perform ! ReadableByteStreamControllerProcessReadRequestsUsingQueue(controller).
        readableByteStreamControllerProcessReadRequestsUsingQueue(controller);
        // 9.2 If ! ReadableStreamGetNumReadRequests(stream) is 0,
        if (readableStreamGetNumReadRequests(stream) === 0) {
            // 9.2.1. Assert: controller.[[pendingPullIntos]] is empty.
            assert(
                controller[_pendingPullIntos].length === 0,
                "Pending pull intos is not empty",
            );
            // 9.2.2. Perform ! ReadableByteStreamControllerEnqueueChunkToQueue(controller, transferredBuffer, byteOffset, byteLength).
            readableByteStreamControllerEnqueueChunkToQueue(
                controller,
                transferredBuffer,
                byteOffset,
                byteLength,
            );
        } else {
            // 9.3 Otherwise,
            // 9.3.1. Assert: controller.[[queue]] is empty.
            assert(controller[_queue].size === 0, "Queue is not empty");
            // 9.3.2. If controller.[[pendingPullIntos]] is not empty,
            if (controller[_pendingPullIntos].length > 0) {
                // 9.3.2.1 Assert: controller.[[pendingPullIntos]][0]'s reader type is "default".
                assert(
                    controller[_pendingPullIntos][0].readerType === "default",
                    "Reader type is not default",
                );
                // 9.3.2.2 Perform ! ReadableByteStreamControllerShiftPendingPullInto(controller).
                readableByteStreamControllerShiftPendingPullInto(controller);
            }
            // 9.3.3. Let transferredView be ! Construct(%Uint8Array%, « transferredBuffer, byteOffset, byteLength »).
            const transferredView = new Uint8Array(
                transferredBuffer,
                byteOffset,
                byteLength,
            );
            // 9.3.4. Perform ! ReadableStreamFulfillReadRequest(stream, transferredView, false).
            readableStreamFulfillReadRequest(stream, transferredView, false);
        }
        // 10. Otherwise, if ! ReadableStreamHasBYOBReader(stream) is true,
    } else if (readableStreamHasBYOBReader(stream)) {
        // 10.1. Perform ! ReadableByteStreamControllerEnqueueChunkToQueue(controller, transferredBuffer, byteOffset, byteLength).
        readableByteStreamControllerEnqueueChunkToQueue(
            controller,
            transferredBuffer,
            byteOffset,
            byteLength,
        );
        // 10.2 Perform ! ReadableByteStreamControllerProcessPullIntoDescriptorsUsingQueue(controller).
        readableByteStreamControllerProcessPullIntoDescriptorsUsingQueue(
            controller,
        );
    } else {
        // 11. Otherwise,
        // 11.1. Assert: ! IsReadableStreamLocked(stream) is false.
        assert(isReadableStreamLocked(stream) === false, "Stream is locked");
        // 11.2. Perform ! ReadableByteStreamControllerEnqueueChunkToQueue(controller, transferredBuffer, byteOffset, byteLength).
        readableByteStreamControllerEnqueueChunkToQueue(
            controller,
            transferredBuffer,
            byteOffset,
            byteLength,
        );
    }
    // 12. Perform ! ReadableByteStreamControllerCallPullIfNeeded(controller).
    readableByteStreamControllerCallPullIfNeeded(controller);
};

const readableStreamCloseByteController = (stream: ReadableStream) => {
    const controller = stream[_controller] as ReadableByteStreamController;
    if (controller[_closeRequested] === true)
        throw new TypeError("Close requested is true");
    // 2. If this.[[stream]].[[state]] is not "readable", throw a TypeError exception.
    if (controller[_stream][_state] !== _readable)
        throw new TypeError("Stream is not readable");
    // 3. Perform ? ReadableByteStreamControllerClose(this).
    readableByteStreamControllerClose(controller);
};

const createReadableByteStream = (
    startAlgorithm: () => void,
    pullAlgorithm: () => Promise<void>,
    cancelAlgorithm: (reason: any) => Promise<void>,
    highWaterMark: number = 0,
) => {
    // 1. Let stream be a new ReadableStream.
    const stream = new ReadableStream();
    // 2. Perform ! InitializeReadableStream(stream).
    initializeReadableStream(stream);
    // 3. Let controller be a new ReadableByteStreamController.
    const controller = new ReadableByteStreamController();
    // 4. Perform ? SetUpReadableByteStreamController(stream, controller, startAlgorithm, pullAlgorithm, cancelAlgorithm, 0, undefined).
    setUpReadableByteStreamController(
        stream,
        controller,
        startAlgorithm,
        pullAlgorithm,
        cancelAlgorithm,
        highWaterMark,
        undefined,
    );
    // 5. Return stream.
    return stream;
};

//x https://streams.spec.whatwg.org/#readablebytestreamcontroller
class ReadableByteStreamController {
    [_autoAllocateChunkSize]?: number;
    [_byobRequest]: ReadableStreamBYOBRequest | null;
    [_cancelAlgorithm]?: (reason: any) => Promise<void>;
    [_closeRequested]: boolean;
    [_pullAgain]: boolean;
    [_pullAlgorithm]?: () => Promise<void>;
    [_pulling]: boolean;
    [_pendingPullIntos]: PullIntoDescriptor[];
    [_queue]: Queue<ArrayBufferView>;
    [_queueTotalSize]: number;
    [_started]: boolean;
    [_strategyHWM]: number;
    [_stream]: ReadableStream;

    get byobRequest() {
        //x
        // 1. Return ! ReadableByteStreamControllerGetBYOBRequest(this).
        return readableByteStreamControllerGetBYOBRequest(this);
    }

    get desiredSize() {
        //x
        // 1. Return ! ReadableByteStreamControllerGetDesiredSize(this).
        return readableByteStreamControllerGetDesiredSize(this);
    }

    close(): void {
        //x
        // 1. If this.[[closeRequested]] is true, throw a TypeError exception.
        if (this[_closeRequested] === true)
            throw new TypeError("Close requested is true");
        // 2. If this.[[stream]].[[state]] is not "readable", throw a TypeError exception.
        if (this[_stream][_state] !== "readable")
            throw new TypeError("Stream is not readable");
        // 3. Perform ? ReadableByteStreamControllerClose(this).
        readableByteStreamControllerClose(this);
    }

    enqueue(chunk: ArrayBufferView): void {
        //x
        // 1. If chunk.[[ByteLength]] is 0, throw a TypeError exception.
        if (chunk.byteLength === 0)
            throw new TypeError("Chunk byteLength is 0");
        // 2. If chunk.[[ViewedArrayBuffer]].[[ArrayBufferByteLength]] is 0, throw a TypeError exception.
        if (chunk.buffer.byteLength === 0)
            throw new TypeError("Chunk buffer byteLength is 0");
        // 3. If this.[[closeRequested]] is true, throw a TypeError exception.
        if (this[_closeRequested])
            throw new TypeError("Close requested is true");
        // 4. If this.[[stream]].[[state]] is not "readable", throw a TypeError exception.
        if (this[_stream][_state] !== "readable")
            throw new TypeError("Stream is not readable");
        // 5. Return ? ReadableByteStreamControllerEnqueue(this, chunk).
        return readableByteStreamControllerEnqueue(this, chunk);
    }

    error(e: any): void {
        //x
        // 1. Perform ! ReadableByteStreamControllerError(this, e).
        readableByteStreamControllerError(this, e);
    }

    [_cancelSteps](reason: any): Promise<void> {
        //x
        // 1. Perform ! ReadableByteStreamControllerClearPendingPullIntos(this).
        readableByteStreamControllerClearPendingPullIntos(this);
        // 2. Perform ! ResetQueue(this).
        resetQueue(this);
        // 3. Let result be the result of performing this.[[cancelAlgorithm]], passing in reason.
        const result = this[_cancelAlgorithm]!(reason);
        // 4. Perform ! ReadableByteStreamControllerClearAlgorithms(this).
        readableByteStreamControllerClearAlgorithms(this);
        // 5. Return result.
        return result;
    }

    [_pullSteps](readRequest: ReadRequest): void {
        //x
        // 1. Let stream be this.[[stream]].
        const stream = this[_stream];
        // 2. Assert: ! ReadableStreamHasDefaultReader(stream) is true.
        assert(
            readableStreamHasDefaultReader(stream),
            "Stream does not have a default reader",
        );
        // 3. If this.[[queueTotalSize]] > 0,
        if (this[_queueTotalSize] > 0) {
            // 3.1. Assert: ! ReadableStreamGetNumReadRequests(stream) is 0.
            assert(
                readableStreamGetNumReadRequests(stream) === 0,
                "Stream has read requests",
            );
            // 3.2 Perform ! ReadableByteStreamControllerFillReadRequestFromQueue(this, readRequest).
            readableByteStreamControllerFillReadRequestFromQueue(
                this,
                readRequest,
            );
            // 3.3 Return.
            return;
        }
        // 4. Let autoAllocateChunkSize be this.[[autoAllocateChunkSize]].
        const autoAllocateChunkSize = this[_autoAllocateChunkSize];
        // 5. If autoAllocateChunkSize is not undefined,
        if (autoAllocateChunkSize !== undefined) {
            let buffer;
            try {
                // 5.1. Let buffer be Construct(%ArrayBuffer%, « autoAllocateChunkSize »).
                buffer = new ArrayBuffer(autoAllocateChunkSize);
            } catch (e) {
                // 5.2. If buffer is an abrupt completion,
                // 5.2.1. Perform readRequest’s error steps, given buffer.[[Value]].
                readRequest.errorSteps(e);
                // 5.2.2. Return.
                return;
            }
            // 5.3. Let pullIntoDescriptor be a new pull-into descriptor with
            let pullIntoDescriptor: PullIntoDescriptor = {
                buffer,
                bufferByteLength: autoAllocateChunkSize,
                byteOffset: 0,
                byteLength: autoAllocateChunkSize,
                bytesFilled: 0,
                elementSize: 1,
                minimumFill: 1,
                viewConstructor: Uint8Array,
                readerType: "default",
            };
            // 5.4. Append pullIntoDescriptor to this.[[pendingPullIntos]].
            this[_pendingPullIntos].push(pullIntoDescriptor);
        }
        // 6. Perform ! ReadableStreamAddReadRequest(stream, readRequest).
        readableStreamAddReadRequest(stream, readRequest);
        // 7. Perform ! ReadableByteStreamControllerCallPullIfNeeded(this).
        readableByteStreamControllerCallPullIfNeeded(this);
    }

    [_releaseSteps](): void {
        //x
        // 1. If this.[[pendingPullIntos]] is not empty,
        if (this[_pendingPullIntos].length > 0) {
            // 1.1. Let firstPendingPullInto be this.[[pendingPullIntos]][0].
            const firstPendingPullInto = this[_pendingPullIntos][0];
            // 1.2. Set firstPendingPullInto’s reader type to "none".
            firstPendingPullInto.readerType = "none";
            // 1.3. Set this.[[pendingPullIntos]] to the list « firstPendingPullInto ».
            this[_pendingPullIntos] = [firstPendingPullInto];
        }
    }
}

// https://streams.spec.whatwg.org/#readablestreamdefaultcontroller
//                                                                |
//              ReadableStreamDefaultController                   |
// ---------------------------------------------------------------|

//x https://streams.spec.whatwg.org/#readable-stream-default-controller-get-desired-size
function readableStreamDefaultControllerGetDesiredSize(
    controller: ReadableStreamDefaultController,
): number | null {
    // 1. Let state be controller.[[stream]].[[state]].
    const state = controller[_stream][_state];
    // 2. If state is "errored", return null.
    if (state === "errored") return null;
    // 3. If state is "closed", return 0.
    if (state === "closed") return 0;
    // 4. Return controller.[[strategyHWM]] − controller.[[queueTotalSize]].
    return controller[_strategyHWM] - controller[_queueTotalSize];
}

//x https://streams.spec.whatwg.org/#readable-stream-default-controller-can-close-or-enqueue
function readableStreamDefaultControllerCanCloseOrEnqueue(
    controller: ReadableStreamDefaultController,
) {
    // 1. Let state be controller.[[stream]].[[state]].
    const state = controller[_stream][_state];
    // 2. If controller.[[closeRequested]] is false and state is "readable", return true.
    // 3. Otherwise, return false.
    return controller[_closeRequested] === false && state === "readable";
}

//x https://streams.spec.whatwg.org/#readable-stream-default-controller-clear-algorithms
function readableStreamDefaultControllerClearAlgorithms(
    controller: ReadableStreamDefaultController,
) {
    // 1. Set controller.[[pullAlgorithm]] to undefined.
    controller[_pullAlgorithm] = undefined;
    // 2. Set controller.[[cancelAlgorithm]] to undefined.
    controller[_cancelAlgorithm] = undefined;
    // 3. Set controller.[[strategySizeAlgorithm]] to undefined.
    controller[_strategySizeAlgorithm] = undefined;
}

//x https://streams.spec.whatwg.org/#readable-stream-default-controller-close
function readableStreamDefaultControllerClose(
    controller: ReadableStreamDefaultController,
) {
    // 1. If ! ReadableStreamDefaultControllerCanCloseOrEnqueue(controller) is false, return.
    if (
        readableStreamDefaultControllerCanCloseOrEnqueue(controller) === false
    ) {
        return;
    }

    // 2. Let stream be controller.[[stream]].
    const stream = controller[_stream];
    // 3. Set controller.[[closeRequested]] to true.
    controller[_closeRequested] = true;

    // 4. If controller.[[queue]] is empty, then
    if (controller[_queue].size === 0) {
        // 4.1. Perform ! ReadableStreamClose(stream).
        readableStreamDefaultControllerClearAlgorithms(controller);
        // 4.2. Perform ! ReadableStreamClose(stream).
        readableStreamClose(stream);
    }
}

//x https://streams.spec.whatwg.org/#reset-queue
function resetQueue<T>(container: QueueContainer<T>) {
    // 1. Assert: container has [[queue]] and [[queueTotalSize]] internal slot.
    // 2. Set container.[[queue]] to a new empty List.
    container[_queue] = new Queue();
    // 3. Set container.[[queueTotalSize]] to 0.
    container[_queueTotalSize] = 0;
}

//x https://streams.spec.whatwg.org/#enqueue-value-with-size
function enqueueValueWithSize(
    container: QueueContainer<ValueWithSize>,
    value: any,
    size: number,
) {
    // 1. Assert: container has [[queue]] and [[queueTotalSize]] internal slots.
    assert(container[_queue] !== undefined);
    assert(typeof container[_queueTotalSize] === "number");
    // 2. If ! IsNonNegativeNumber(size) is false, throw a RangeError exception.
    if (typeof size !== "number" || size < 0)
        throw new RangeError("Size must be a non-negative number");
    // 3. If size is +∞, throw a RangeError exception.
    if (size === Infinity) throw new RangeError("Size must not be Infinity");
    // 4. Append a new value-with-size with value value and size size to container.[[queue]].
    container[_queue].enqueue({ value, size });
    // 5. Set container.[[queueTotalSize]] to container.[[queueTotalSize]] + size.
    container[_queueTotalSize] += size;
}

//x https://streams.spec.whatwg.org/#dequeue-value
function dequeueValue(container: QueueContainer) {
    // 1. Assert: container has [[queue]] and [[queueTotalSize]] internal slots.
    assert(container[_queue] !== undefined);
    assert(typeof container[_queueTotalSize] === "number");
    // 2. Assert: container.[[queue]] is not empty.
    if (container[_queue].isEmpty()) {
        throw new TypeError("Queue is empty");
    }
    // 3. Let valueWithSize be container.[[queue]][0].
    // 4. Remove valueWithSize from container.[[queue]].
    const valueWithSize = container[_queue].dequeue();
    // 5. Set container.[[queueTotalSize]] to container.[[queueTotalSize]] − valueWithSize’s size.
    container[_queueTotalSize] -= valueWithSize.size;
    // 6. If container.[[queueTotalSize]] < 0, set container.[[queueTotalSize]] to 0. (This can occur due to rounding errors.)
    if (container[_queueTotalSize] < 0) {
        container[_queueTotalSize] = 0;
    }
    // 7. Return valueWithSize’s value.
    return valueWithSize.value;
}

//x https://streams.spec.whatwg.org/#readable-stream-default-controller-should-call-pull
function readableStreamDefaultControllerShouldCallPull(
    controller: ReadableStreamDefaultController,
): boolean {
    // 1. Let stream be controller.[[stream]].
    const stream = controller[_stream];
    // 2. If ! ReadableStreamDefaultControllerCanCloseOrEnqueue(controller) is false, return false.
    if (readableStreamDefaultControllerCanCloseOrEnqueue(controller) === false)
        return false;
    // 3. If controller.[[started]] is false, return false.
    if (controller[_started] === false) return false;
    // 4. If ! IsReadableStreamLocked(stream) is true and ! ReadableStreamGetNumReadRequests(stream) > 0, return true.
    if (
        isReadableStreamLocked(stream) &&
        readableStreamGetNumReadRequests(stream) > 0
    )
        return true;
    // 5. Let desiredSize be ! ReadableStreamDefaultControllerGetDesiredSize(controller).
    const desiredSize =
        readableStreamDefaultControllerGetDesiredSize(controller);
    // 6. Assert: desiredSize is not null.
    assert(desiredSize !== null, "Desired size is null");
    // 7. If desiredSize > 0, return true.
    if (desiredSize! > 0) return true;
    // 8. Return false.
    return false;
}

//x https://streams.spec.whatwg.org/#readable-stream-default-controller-call-pull-if-needed
function readableStreamDefaultControllerCallPullIfNeeded(
    controller: ReadableStreamDefaultController,
): void {
    // 1. Let shouldPull be ! ReadableStreamDefaultControllerShouldCallPull(controller).
    // 2. If shouldPull is false, return.
    if (readableStreamDefaultControllerShouldCallPull(controller) === false)
        return;
    // 3. If controller.[[pulling]] is true,
    if (controller[_pulling] === true) {
        // 3.1. Set controller.[[pullAgain]] to true.
        controller[_pullAgain] = true;
        // 3.2. Return.
        return;
    }
    // 4. Assert: controller.[[pullAgain]] is false.
    if (controller[_pullAgain] !== false)
        throw new TypeError("PullAgain is true");
    // 5. Set controller.[[pulling]] to true.
    controller[_pulling] = true;
    // 6. Let pullPromise be the result of performing controller.[[pullAlgorithm]].
    const pullPromise = controller[_pullAlgorithm]!();
    // 7. Upon fulfillment of pullPromise,
    pullPromise.then(
        () => {
            // 7.1. Set controller.[[pulling]] to false.
            controller[_pulling] = false;
            // 7.2. If controller.[[pullAgain]] is true,
            if (controller[_pullAgain] === true) {
                // 7.2.1. Set controller.[[pullAgain]] to false.
                controller[_pullAgain] = false;
                // 7.2.2. Perform ! ReadableStreamDefaultControllerCallPullIfNeeded(controller).
                readableStreamDefaultControllerCallPullIfNeeded(controller);
            }
        },
        // 8. Upon rejection of pullPromise with reason e,
        // 8.1 Perform ! ReadableStreamDefaultControllerError(controller, e).
        (e) => readableStreamDefaultControllerError(controller, e),
    );
}

//x https://streams.spec.whatwg.org/#readable-stream-add-read-request
function readableStreamAddReadRequest(
    stream: ReadableStream,
    readRequest: ReadRequest,
) {
    // 1. Assert: stream.[[reader]] implements ReadableStreamDefaultReader.
    if (!isReadableStreamDefaultReader(stream[_reader]))
        throw new TypeError("Reader is not a ReadableStreamDefaultReader");
    // 2. Assert: stream.[[state]] is "readable".
    if (stream[_state] !== "readable")
        throw new TypeError("Stream is not readable");
    // 3. Append readRequest to stream.[[reader]].[[readRequests]].
    stream[_reader][_readRequests].push(readRequest);
}

//x https://streams.spec.whatwg.org/#readable-stream-fulfill-read-request
function readableStreamFulfillReadRequest(
    stream: ReadableStream,
    chunk: any,
    done: boolean,
) {
    // 1. Assert: ! ReadableStreamHasDefaultReader(stream) is true.
    if (!readableStreamHasDefaultReader(stream))
        throw new TypeError("Stream has no default reader");
    // 2. Let reader be stream.[[reader]].
    const reader = stream[_reader] as ReadableStreamDefaultReader;
    // 3. Assert: reader.[[readRequests]] is not empty.
    if (reader[_readRequests].length === 0)
        throw new TypeError("Read requests is empty");
    // 4. Let readRequest be reader.[[readRequests]][0].
    const readRequest = reader[_readRequests][0];
    // 5. Remove readRequest from reader.[[readRequests]].
    reader[_readRequests].shift();
    // 6. If done is true, perform readRequest’s close steps.
    if (done === true) readRequest.closeSteps();
    // 7. Otherwise, perform readRequest’s chunk steps, given chunk.
    else readRequest.chunkSteps(chunk);
}

//x https://streams.spec.whatwg.org/#readable-stream-default-controller-error
function readableStreamDefaultControllerError(
    controller: ReadableStreamDefaultController,
    e: any,
) {
    // 1. Let stream be controller.[[stream]].
    const stream = controller[_stream];
    // 2. If stream.[[state]] is not "readable", then return.
    if (stream[_state] !== "readable") return;
    // 3. Perform ! ResetQueue(controller).
    resetQueue(controller);
    // 4. Perform ! ReadableStreamDefaultControllerClearAlgorithms(controller).
    readableStreamDefaultControllerClearAlgorithms(controller);
    // 5. Perform ! ReadableStreamError(stream, e).
    readableStreamError(stream, e);
}

//x https://streams.spec.whatwg.org/#readable-stream-default-controller-enqueue
function readableStreamDefaultControllerEnqueue(
    controller: ReadableStreamDefaultController,
    chunk: any,
) {
    // 1. If ! ReadableStreamDefaultControllerCanCloseOrEnqueue(controller) is false, return.
    if (readableStreamDefaultControllerCanCloseOrEnqueue(controller) === false)
        return;
    // 2. Let stream be controller.[[stream]].
    const stream = controller[_stream];
    // 3. If ! IsReadableStreamLocked(stream) is true and ! ReadableStreamGetNumReadRequests(stream) > 0, perform ! ReadableStreamFulfillReadRequest(stream, chunk, false).
    if (
        isReadableStreamLocked(stream) &&
        readableStreamGetNumReadRequests(stream) > 0
    ) {
        readableStreamFulfillReadRequest(stream, chunk, false);
    } else {
        // 4.1 Let result be the result of performing controller.[[strategySizeAlgorithm]], passing in chunk, and interpreting the result as a completion record.
        let chunkSize = 1;
        try {
            // 4.3 Let chunkSize be result.[[Value]].
            chunkSize = controller[_strategySizeAlgorithm]!(chunk);
            // 4.4 Let enqueueResult be EnqueueValueWithSize(controller, chunk, chunkSize).
            enqueueValueWithSize(controller, chunk, chunkSize);
        } catch (e) {
            // 4.2 If result is an abrupt completion,
            // 4.2.1. Perform ! ReadableStreamDefaultControllerError(controller, result.[[Value]]).
            readableStreamDefaultControllerError(controller, e);
            throw e;
        }
    }
    // 5. Perform ! ReadableStreamDefaultControllerCallPullIfNeeded(controller).
    readableStreamDefaultControllerCallPullIfNeeded(controller);
}

//x https://streams.spec.whatwg.org/#readablestreamdefaultcontroller
class ReadableStreamDefaultController {
    [_cancelAlgorithm]?: (reason: any) => Promise<any>;
    [_closeRequested]: boolean;
    [_pullAgain]: boolean;
    [_pullAlgorithm]?: () => Promise<void>;
    [_pulling]: boolean;
    [_queue]: Queue<ValueWithSize>;
    [_queueTotalSize]: number;
    [_started]: boolean;
    [_strategyHWM]: number;
    [_strategySizeAlgorithm]?: QueuingStrategySizeCallback;
    [_stream]: ReadableStream;

    get desiredSize() {
        assert(isPrototypeOf(this, ReadableStreamDefaultController.prototype));
        return readableStreamDefaultControllerGetDesiredSize(this);
    }

    close() {
        assert(isPrototypeOf(this, ReadableStreamDefaultController.prototype));

        if (!readableStreamDefaultControllerCanCloseOrEnqueue(this)) {
            throw new TypeError(
                "The stream is not in a state that permits close",
            );
        }

        readableStreamDefaultControllerClose(this);
    }

    error(e: any) {
        assert(isPrototypeOf(this, ReadableStreamDefaultController.prototype));
        readableStreamDefaultControllerError(this, e);
    }

    enqueue(chunk: any) {
        assert(isPrototypeOf(this, ReadableStreamDefaultController.prototype));
        if (!readableStreamDefaultControllerCanCloseOrEnqueue(this)) {
            throw new TypeError(
                "The stream is not in a state that permits enqueue",
            );
        }

        readableStreamDefaultControllerEnqueue(this, chunk);
    }

    [Symbol.toStringTag] = "ReadableStreamDefaultController";

    [_cancelSteps](reason: any) {
        //x
        // 1. Perform ! ResetQueue(this).
        resetQueue(this);
        // 2. Let result be the result of performing this.[[cancelAlgorithm]], passing reason.
        const result = this[_cancelAlgorithm]!(reason);
        // 3. Perform ! ReadableStreamDefaultControllerClearAlgorithms(this).
        readableStreamDefaultControllerClearAlgorithms(this);
        // 4. Return result.
        return result;
    }

    [_pullSteps](readRequest: ReadRequest) {
        //x
        // 1. Let stream be this.[[stream]].
        const stream = this[_stream];
        // 2. If this.[[queue]] is not empty,
        if (this[_queue].size > 0) {
            // 2.1. Let chunk be ! DequeueValue(this).
            const chunk = dequeueValue(this);
            // 2.2 If this.[[closeRequested]] is true and this.[[queue]] is empty,
            if (this[_closeRequested] === true && this[_queue].isEmpty()) {
                // 2.2.1 Perform ! ReadableStreamDefaultControllerClearAlgorithms(this).
                readableStreamDefaultControllerClearAlgorithms(this);
                // 2.2.2 Perform ! ReadableStreamClose(stream).
                readableStreamClose(stream);
            } else {
                // 2.3 Otherwise, perform ! ReadableStreamDefaultControllerCallPullIfNeeded(this).
                readableStreamDefaultControllerCallPullIfNeeded(this);
            }
            // 2.4 Perform readRequest’s chunk steps, given chunk.
            readRequest.chunkSteps(chunk);
        } else {
            // 3.1 Perform ! ReadableStreamAddReadRequest(stream, readRequest).
            readableStreamAddReadRequest(stream, readRequest);
            // 3.2 Perform ! ReadableStreamDefaultControllerCallPullIfNeeded(this).
            readableStreamDefaultControllerCallPullIfNeeded(this);
        }
    }

    [_releaseSteps]() {
        //x
        // 1. Return.
        return;
    }
}

export {
    ByteLengthQueuingStrategy,
    CountQueuingStrategy,
    createReadableByteStream,
    createReadableStream,
    isDisturbed,
    isErrored,
    isInReadableState,
    ReadableByteStreamController,
    ReadableStream,
    ReadableStreamBYOBReader,
    ReadableStreamBYOBRequest,
    readableStreamClose,
    readableStreamCloseByteController,
    ReadableStreamDefaultController,
    ReadableStreamDefaultReader,
    readableStreamEnqueue,
    ReadableStreamReaderMode,
    readableStreamResource,
    ReadableStreamType,
};
