type ReadableStreamState = "readable" | "closed" | "errored";

interface ReadRequest<T = any> {
  chunkSteps: (chunk: T) => void;
  closeSteps: () => void;
  errorSteps: (e: any) => void;
}

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
  mode?: "byob";
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
  type?: "bytes";
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
