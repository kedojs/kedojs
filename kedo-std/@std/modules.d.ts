declare module "@kedo/internal/utils" {
  export function is_array_buffer_detached(buffer: ArrayBufferLike): boolean;
  export function parse_url_encoded_form(body: string): [string, string][];
  export function serialize_url_encoded_form(data: [string, string][]): string;
  export function encoding_for_label_no_replacement(label: string): string;
  export function encoding_decode(
    decoder: EncodingTextDecoder,
    buffer: ArrayBuffer,
    stream?: boolean,
  ): string;
  export function encoding_encode(input: string): Uint8Array;
  export function encoding_decode_once(
    buffer: ArrayBuffer,
    label: string,
    fatal: boolean,
    ignoreBOM?: boolean,
  ): string;
  export function encoding_decode_utf8_once(
    buffer: ArrayBuffer,
    ignoreBOM?: boolean,
  ): string;
  export function queue_internal_timeout(
    callback: (...args: any[]) => void,
    delay: number,
    ...args: any[]
  ): void;
  export class UrlRecord {
    constructor(url: string, base?: string);
    get(key: string): string | null;
    set(key: string, value: string): void;
    toString(): string;
  }

  export class EncodingTextDecoder {
    constructor(label: string, fatal: boolean, ignoreBOM: boolean);
  }
}

declare module "@kedo/ds" {
  class Queue<T> {
    constructor();
    enqueue(value: T): void;
    dequeue(): T;
    peek(): T;
    isEmpty(): boolean;
    clear(): void;
    get size(): number;
  }

  class IterableWeakSet<T extends object> {
    constructor();
    add(value: T): IterableWeakSet<T>;
    delete(value: T): boolean;
    has(value: T): boolean;
    [Symbol.iterator](): Iterator<T>;
    clear(): void;
    size(): number;
  }
}

declare module "@kedo/events" {
  class Event {
    constructor(type: string, eventInitDict?: EventInit);
    readonly type: string;
    readonly bubbles: boolean;
    readonly cancelable: boolean;
    readonly defaultPrevented: boolean;
    readonly eventPhase: number;
    readonly target: EventTarget | null;
    readonly currentTarget: EventTarget | null;
    stopPropagation(): void;
    stopImmediatePropagation(): void;
    preventDefault(): void;
  }

  class EventTarget {
    addEventListener(type: string, listener: Listener): void;
    removeEventListener(type: string, listener: Listener): void;
    dispatchEvent(event: Event): boolean;
  }

  type EventListener = (event: Event) => void;
  class EventEmitter {
    static errorMonitor: symbol;
    static errorEvent: string;

    on(event: string | symbol, listener: Listener): EventEmitter;
    once(event: string | symbol, listener: Listener): EventEmitter;
    off(event: string | symbol, listener: Listener): EventEmitter;
    removeAllListeners(event?: string | symbol): void;
    emit(event: string | symbol, ...args: any[]): boolean;
    listenerCount(event: string | symbol): number;
    eventNames(): Array<string | symbol>;
    setMaxListeners(n: number): void;
    getMaxListeners(): number;
  }
}

type Listener = (...args: any[]) => void | Promise<void>;

interface EventInit {
  bubbles?: boolean;
  cancelable?: boolean;
  composed?: boolean;
}

type BodyInit =
  | Blob
  | BufferSource
  | ArrayBufferLike
  | ArrayBufferView
  | FormData
  | URLSearchParams
  | ReadableStream<Uint8Array>
  | string;

interface RequestInit {
  method?: string;
  headers?: Headers | [string, string][] | Record<string, string>;
  body?: BodyInit | null;
  referrer?: string;
  referrerPolicy?: ReferrerPolicy;
  mode?: RequestMode;
  requestCredentials?: RequestCredentials;
  cache?: RequestCache;
  redirect?: RequestRedirect;
  integrity?: string;
  keepalive?: boolean;
  signal?: AbortSignal | null;
  duplex?: RequestDuplex;
  priority?: RequestPriority;
}

type ReferrerPolicy =
  | ""
  | "no-referrer"
  | "no-referrer-when-downgrade"
  | "same-origin"
  | "origin"
  | "strict-origin"
  | "origin-when-cross-origin"
  | "strict-origin-when-cross-origin"
  | "unsafe-url";

type RequestDestination =
  | ""
  | "audio"
  | "audioworklet"
  | "document"
  | "embed"
  | "font"
  | "frame"
  | "iframe"
  | "image"
  | "json"
  | "manifest"
  | "object"
  | "paintworklet"
  | "report"
  | "script"
  | "sharedworker"
  | "style"
  | "track"
  | "video"
  | "worker"
  | "xslt";

type RequestMode = "navigate" | "same-origin" | "no-cors" | "cors";

type RequestCredentials = "omit" | "same-origin" | "include";

type RequestCache =
  | "default"
  | "no-store"
  | "reload"
  | "no-cache"
  | "force-cache"
  | "only-if-cached";

type RequestRedirect = "follow" | "error" | "manual";

type RequestDuplex = "half";

type RequestPriority = "high" | "low" | "auto";

// declare module "@kedo/assert" {
// }

declare module "@kedo/stream" {
  class ReadableStream {
    readonly locked: boolean;
    constructor(
      underlyingSource: UnderlyingSource | null,
      strategy?: QueuingStrategy,
    );
    cancel(reason: any): Promise<void>;
    static from<T>(iterable: Iterable<T> | AsyncIterable<T>): ReadableStream;
    getReader<
      T = ReadableStreamDefaultReader | ReadableStreamBYOBReader,
    >(options?: { mode: "byob" }): T;
    values<T>(args: { preventCancel?: boolean }): AsyncIterableIterator<any>;
    [Symbol.asyncIterator]<T>(): AsyncIterableIterator<T>;
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
  export function readableStreamClose(stream: ReadableStream): void;
}

// import { Headers } from "../../types/Headers";

declare module "@kedo/web/internals" {
  // export { Headers };
}
