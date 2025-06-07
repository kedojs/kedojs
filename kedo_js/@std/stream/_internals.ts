export {
    ByteLengthQueuingStrategy,
    CountQueuingStrategy,
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
    readableStreamResource,
} from "./Readable";

export const StreamError = {
    Closed: -1.0,
    ChannelFull: -2.0,
    ReceiverTaken: -3.0,
    SendError: -4.0,
    Empty: -5.0,
};
