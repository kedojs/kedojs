declare type HeadersGuard =
    | "none"
    | "immutable"
    | "request"
    | "request-no-cors"
    | "response";

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

declare type PropertyNameKind = "KeyAndValue" | "Key" | "Value";

interface ExtractedBody {
    stream: import("@kedo/stream").ReadableStream | null;
    source: Uint8Array | null;
    length: number | null;
    type: string | null;
}
interface TextDecoderOptions {
    fatal?: boolean;
    ignoreBOM?: boolean;
}

type MixingBodyInput = import("@kedo:int/std/web").Request | import("@kedo:int/std/web").Response;

interface TextDecodeOptions {
    stream?: boolean;
}

declare type TextDecodeInput = ArrayBufferView | ArrayBuffer | DataView;

declare type RequestInfo = import("@kedo:int/std/web").Request | string;
declare type ResponseTainting = "basic" | "cors" | "opaque";
declare type ResponseType = "basic" | "cors" | "default" | "error" | "opaque" | "opaqueredirect";
declare type BodyInit = Blob | BufferSource | FormData | import("@kedo:int/std/web").URLSearchParams | ArrayBufferLike | ArrayBufferView | import("@kedo/stream").ReadableStream | string;

declare type ServeOptions = {
    hostname?: string;
    port?: number;
    signal?: import("@kedo:int/std/web").AbortSignal;
    onListen?: (args: any) => void;
    handler?: ServerHandler;
};

declare type TlsCertificate = {
    keyFormat: "pem" | "der";
    key: string;
    cert: string;
};

declare type ServerHandler = (request: import("@kedo:int/std/web").Request) => Promise<import("@kedo:int/std/web").Response>;

type HeadersInit = import("@kedo:int/std/web").Headers | Record<string, string> | [string, string][];

interface ResponseInit {
    status?: number;
    statusText?: string;
    headers?: HeadersInit;
}

interface RequestInit {
    method?: string;
    headers?: HeadersInit;
    body?: BodyInit | null;
    referrer?: string;
    referrerPolicy?: ReferrerPolicy;
    mode?: RequestMode;
    requestCredentials?: RequestCredentials;
    cache?: RequestCache;
    redirect?: RequestRedirect;
    integrity?: string;
    keepalive?: boolean;
    signal?: import("@kedo:int/std/web").AbortSignal | null;
    duplex?: RequestDuplex;
    priority?: RequestPriority;
}

interface IRequest {
    url: string;
    method: string;
    headers: Headers;
    referrer: string;
    referrerPolicy: ReferrerPolicy;
    mode: RequestMode;
    credentials: RequestCredentials;
    cache: RequestCache;
    redirect: RequestRedirect;
    integrity: string;
    keepalive: boolean;
    signal: AbortSignal;
    duplex: string;
    body: ReadableStream | null;
    bodyUsed: boolean;
    bytes(): Promise<Uint8Array>;
    text(): Promise<string>;
    json(): Promise<any>;
    arrayBuffer(): Promise<ArrayBuffer>;
    clone(): IRequest;
}

interface IResponse {
    type: ResponseType;
    url: string;
    redirected: boolean;
    status: number;
    ok: boolean;
    statusText: string;
    headers: Headers;
    clone(): Response;
    body: ReadableStream | null;
    arrayBuffer(): Promise<ArrayBuffer>;
    json(): Promise<any>;
    text(): Promise<string>;
}

declare module "@kedo/web" {
    export {
        AbortController,
        AbortSignal, DOMException, fetch, Headers, Request,
        Response,
        serve, TextDecoder,
        TextEncoder, URL,
        URLSearchParams
    } from "@kedo:int/std/web";
}

declare module "@kedo:int/std/web" {

    import { EventTarget } from "@kedo/events";
    type ForEachCallback = (
        value: string,
        name: string,
        headers: Headers,
    ) => void;

    const fillHeadersMapFrom: (
        headers: HeadersInit,
        headersMap: Headers,
        headersGuard?: HeadersGuard,
    ) => void;

    const headerInnerList: (headers: Headers) => [string, string][];

    /**
     * Represents a collection of HTTP headers with methods to manage
     * header name-value pairs, including support for adding, retrieving,
     * and removing values.
     *
     * @remarks
     * The class also facilitates iteration over the headers and provides
     * support for retrieving set-cookie values.
     *
     * @example
     * ```ts
     * const headers = new Headers({ 'Content-Type': 'application/json' });
     * headers.append('Authorization', 'Bearer token');
     * console.log(headers.get('Authorization')); // 'Bearer token'
     * ```
     */
    class Headers {
        constructor(init: HeadersInit);
        append(name: string, value: string): void;
        delete(name: string): void;
        get(name: string): string | null;
        has(name: string): boolean;
        set(name: string, value: string): void;
        forEach(callback: ForEachCallback, thisArg?: any): void;
        [Symbol.toStringTag]: string;
        [Symbol.iterator](): {
            [Symbol.iterator](): any;
            [Symbol.toStringTag]: string;
            next: () => IteratorResult<string | [string, string] | undefined>;
        };
        getSetCookie(): string[];
        entries(): IterableIterator<[string, string]>;
        keys(): IterableIterator<string>;
        values(): IterableIterator<string>;
    }

    const emptyHeader: (headersMap: Headers) => void;

    /**
     * Represents an exception that is thrown when a DOM-related error occurs.
     * Extends the built-in Error object with a numeric code and other DOM-specific properties.
     * 
     * @public
     * @remarks
     * Inspired by the browser DOMException interface, it contains legacy codes and names
     * to maintain compatibility. Modern usage may rely more on the message and name properties.
     */
    class DOMException extends Error {
        /**
         * A string identifying the type of error.
         * 
         * @public
         */
        readonly name: string;

        /**
         * Provides details about the exception that was raised.
         * 
         * @public
         */
        readonly message: string;
        /**
         * A legacy numeric code corresponding to the type of error, retained for compatibility.
         * 
         * @public
         */
        readonly code: number;
        static readonly INDEX_SIZE_ERR = 1;
        static readonly DOMSTRING_SIZE_ERR = 2;
        static readonly HIERARCHY_REQUEST_ERR = 3;
        static readonly WRONG_DOCUMENT_ERR = 4;
        static readonly INVALID_CHARACTER_ERR = 5;
        static readonly NO_DATA_ALLOWED_ERR = 6;
        static readonly NO_MODIFICATION_ALLOWED_ERR = 7;
        static readonly NOT_FOUND_ERR = 8;
        static readonly NOT_SUPPORTED_ERR = 9;
        static readonly INUSE_ATTRIBUTE_ERR = 10;
        static readonly INVALID_STATE_ERR = 11;
        static readonly SYNTAX_ERR = 12;
        static readonly INVALID_MODIFICATION_ERR = 13;
        static readonly NAMESPACE_ERR = 14;
        static readonly INVALID_ACCESS_ERR = 15;
        static readonly VALIDATION_ERR = 16;
        static readonly TYPE_MISMATCH_ERR = 17;
        static readonly SECURITY_ERR = 18;
        static readonly NETWORK_ERR = 19;
        static readonly ABORT_ERR = 20;
        static readonly URL_MISMATCH_ERR = 21;
        static readonly QUOTA_EXCEEDED_ERR = 22;
        static readonly TIMEOUT_ERR = 23;
        static readonly INVALID_NODE_TYPE_ERR = 24;
        static readonly DATA_CLONE_ERR = 25;

        /**
         * Several static properties representing standard error codes used by DOM specifications.
         * 
         * @public
         */
        private static readonly errorCodes;

        /**
         * Constructs a new DOMException.
         * 
         * @param message - Optional error message summarizing the exception.
         * @param name - Optional error name. Defaults to 'Error' if not specified.
         * 
         * @public
         */
        constructor(message?: string, name?: string);

        /**
         * Serializes the DOMException into a plain object.
         * 
         * @returns An object with the exception's name, message, and code.
         * @public
         */
        toJSON(): {
            name: string;
            message: string;
            code: number;
        };

        /**
         * Recreates a DOMException from a serialized object.
         * 
         * @param serialized - Object containing at least a name and message property.
         * @returns A new DOMException populated with the serialized data.
         * @public
         */
        static fromJSON(serialized: {
            name: string;
            message: string;
        }): DOMException;
    }

    interface AbortAlgorithm {
        (): void;
    }

    /**
     * Represents a signal object that can be used to communicate abort requests.
     * It extends the capabilities of an `EventTarget` to dispatch an `abort` event
     * when an underlying operation is canceled or signaled to end.
     *
     * @remarks
     * Instances of this class track the reason for the abortion, manage internal
     * algorithms that determine how and when to abort, and coordinate with other
     * signals that may depend on or propagate the abort event.
     *
     * @public
     *
     * @property aborted
     * Whether the signal has been aborted.
     *
     * @property reason
     * Holds the reason for the abort, if provided.
     *
     * @method throwIfAborted
     * Throws an error if the signal has been aborted. Useful in operations
     * where early termination is necessary once an abort is requested.
     *
     * @method static abort
     * Creates a new `AbortSignal` that is already aborted with the given reason.
     *
     * @method static timeout
     * Creates a new `AbortSignal` that will automatically abort after a specified
     * timeout in milliseconds.
     *
     * @method static any
     * Creates a new `AbortSignal` that is aborted as soon as any signal in the
     * provided array becomes aborted.
     *
     * @event onabort
     * Event listener triggered when the signal is aborted.
     *
     * @example
     * ```ts
     * const signal = AbortSignal.timeout(5000);
     * signal.onabort = () => {
     *   console.log('Operation aborted due to timeout');
     * };
     * ```
     */
    class AbortSignal extends EventTarget {
        constructor(key?: any);
        get aborted(): boolean;
        get reason(): any;
        throwIfAborted(): void;
        static abort(reason: any): AbortSignal;
        static timeout(ms: number): AbortSignal;
        static any(signals: AbortSignal[]): AbortSignal;
        set onabort(listener: EventListener);
    }

    const createDependentAbortSignal: (signals: AbortSignal[]) => AbortSignal;

    // const _signal: unique symbol;

    /**
     * Provides an object that can abort one or more associated requests.
     *
     * @remarks
     * Each AbortController has an associated AbortSignal, which can be used to
     * observe and react to an abort event. Once the controller has signaled an
     * abort, the signal's `aborted` property becomes `true`.
     *
     * @constructor
     * Creates a new instance of the AbortController, providing a unique
     * signal for abort tracking.
     *
     * @property signal
     * The `AbortSignal` object that is linked to this controller.
     *
     * @method abort
     * Aborts the associated activities, causing the `signal.aborted` property
     * to become `true`.
     * @param reason - An optional reason for triggering the abort. This can
     *                 be an error or any other value describing why the
     *                 operation was canceled.
     */
    class AbortController {
        // [_signal]: AbortSignal;
        constructor();
        get signal(): AbortSignal;
        abort(reason?: any): void;
    }

    const _urlObject: unique symbol;
    const _list: unique symbol;
    /**
     * Represents a collection of key-value pairs corresponding to the query parameters of a URL.
     *
     * @remarks
     * This class provides methods to append, delete, retrieve, and modify URL query parameters.
     * When serialized with {@link toString}, it produces a valid query string.
     */

    /**
     * Creates a new instance of this class, optionally accepting an initialized set of parameters.
     * @param init - An array of [name, value] pairs, an object with key-value pairs, or a query string.
     */

    /**
     * Retrieves the total number of query parameters.
     * @returns The count of all parameters.
     */

    /**
     * Appends a given parameter to the list, preserving any existing parameters with the same name.
     * @param name - The parameter name.
     * @param value - The parameter value to be appended.
     */

    /**
     * Deletes one or all occurrences of a parameter from the list.
     * @param name - The parameter name to remove.
     * @param value - An optional specific value to target; if omitted, all values for the parameter are removed.
     */

    /**
     * Retrieves the first value associated with a parameter.
     * @param name - The parameter name.
     * @returns The first value for the specified parameter, or null if none exists.
     */

    /**
     * Retrieves all values associated with a given parameter.
     * @param name - The parameter name.
     * @returns An array of all values corresponding to the parameter.
     */

    /**
     * Checks if a parameter with the given name (and optional value) exists in the collection.
     * @param name - The parameter name to check.
     * @param value - An optional specific value to verify.
     * @returns True if the parameter is found; otherwise, false.
     */

    /**
     * Sets the value of a parameter, removing any other values for the same parameter.
     * @param name - The parameter name.
     * @param value - The new value for the parameter.
     */

    /**
     * Returns an iterator over all [name, value] pairs in the collection.
     * @returns An iterable iterator yielding parameter name-value pairs.
     */

    /**
     * Returns an iterator over all parameter names in the collection.
     * @returns An iterable iterator yielding parameter names.
     */

    /**
     * Returns an iterator over all parameter values in the collection.
     * @returns An iterable iterator yielding parameter values.
     */

    /**
     * Sorts the parameters in place by their names, comparing them as strings in ascending order.
     */

    /**
     * Produces a query string representing all parameters in standard URL-encoded format.
     * @returns A query string with all name-value pairs.
     */

    /**
     * Returns an iterator over [name, value] pairs, identical to the result of {@link entries}.
     * @returns An iterable iterator of name-value pairs.
     */
    class URLSearchParams {
        [_list]: [string, string][];
        [_urlObject]: URL | null;
        constructor(init?: [string, string][] | Record<string, string> | string);
        private update;
        get size(): number;
        append(name: string, value: string): void;
        delete(name: string, value?: string): void;
        get(name: string): string | null;
        getAll(name: string): string[];
        has(name: string, value?: string): boolean;
        set(name: string, value: string): void;
        entries(): IterableIterator<[string, string]>;
        keys(): IterableIterator<string>;
        values(): IterableIterator<string>;
        sort(): void;
        toString(): string;
        [Symbol.iterator](): IterableIterator<[string, string]>;
    }

    const _urlRecord: unique symbol;
    const _queryObject: unique symbol;
    /**
     * Represents a URL object that provides methods and properties for working with URLs.
     * This class follows the WHATWG URL Standard.
     * 
     * @class
     * @example
     * ```typescript
     * const url = new URL('https://example.com/path?query=value#hash');
     * console.log(url.hostname); // "example.com"
     * console.log(url.searchParams.get('query')); // "value"
     * ```
     * 
     * @property {URLSearchParams} searchParams - Contains the query string parameters
     * @property {string} origin - Returns the origin of the URL (protocol + hostname + port)
     * @property {string} protocol - Gets or sets the protocol scheme of the URL
     * @property {string} username - Gets or sets the username specified in the URL
     * @property {string} password - Gets or sets the password specified in the URL
     * @property {string} host - Gets or sets the host portion of the URL (hostname + port)
     * @property {string} hostname - Gets or sets the hostname portion of the URL
     * @property {string} port - Gets or sets the port number of the URL
     * @property {string} pathname - Gets or sets the path portion of the URL
     * @property {string} search - Gets or sets the query string portion of the URL
     * @property {string} hash - Gets or sets the fragment identifier of the URL
     * @property {string} href - Gets or sets the entire URL as a string
     * 
     * @constructor
     * @param {string} url - The URL string to parse
     * @param {string} [base] - An optional base URL to resolve against
     * 
     * @throws {TypeError} When the URL is invalid or cannot be parsed
     */
    class URL {
        // [_queryObject]: URLSearchParams;
        // [_urlRecord]: import("@kedo:op/web").UrlRecord;
        constructor(url: string, base?: string);
        static parse(url: string, base?: string): URL | null;
        static canParse(url: string, base?: string): boolean;
        get searchParams(): URLSearchParams;
        get origin(): string;
        get protocol(): string;
        set protocol(value: string);
        get username(): string;
        set username(value: string);
        get password(): string;
        set password(value: string);
        get host(): string;
        set host(value: string);
        get hostname(): string;
        set hostname(value: string);
        get port(): string;
        set port(value: string | null);
        get pathname(): string;
        set pathname(value: string);
        get search(): string;
        set search(value: string);
        get hash(): string;
        set hash(value: string);
        toJSON(): string;
        toString(): string;
        get href(): string;
        set href(value: string);
    }

    /**
     * The TextDecoder interface represents a decoder for a specific text encoding.
     * It provides functionality to decode buffer data into strings using specified character encodings.
     * 
     * @see https://developer.mozilla.org/en-US/docs/Web/API/TextDecoder
     * 
     * @example
     * const decoder = new TextDecoder(); // defaults to 'utf-8'
     * const text = decoder.decode(uint8Array);
     */
    class TextDecoder {
        constructor(label?: string, options?: TextDecoderOptions);
        get encoding(): string;
        get fatal(): boolean;
        get ignoreBOM(): boolean;
        decode(input?: TextDecodeInput, options?: TextDecodeOptions): string;
    }

    /**
     * The TextEncoder class represents an encoder that takes a stream of code points as input
     * and emits a stream of bytes. It converts JavaScript strings into bytes using UTF-8 encoding.
     * 
     * @example
     * ```typescript
     * const encoder = new TextEncoder();
     * const bytes = encoder.encode('Hello'); // Returns Uint8Array
     * ```
     * 
     * @remarks
     * TextEncoder only supports UTF-8 encoding.
     */
    class TextEncoder {
        constructor();
        get encoding(): string;
        encode(input?: string): Uint8Array;
    }

    type InnerRequest = {
        method: string;
        url: URL;
        localURLsOnlyFlag?: boolean;
        header_list: [string, string][];
        unsafeRequestFlag?: boolean;
        body: Uint8Array | ExtractedBody | null;
        keepalive: boolean;
        priority: RequestPriority;
        origin: string;
        referrer: string;
        referrerPolicy: ReferrerPolicy;
        mode: RequestMode;
        useCORSPreflightFlag?: boolean;
        redirect: RequestRedirect;
        cache: RequestCache;
        integrity: string;
        credentials: RequestCredentials;
        initiatorType?: "fetch";
        urlList: URL[];
        currentURL: URL;
        redirectCount: number;
        responseTainting: ResponseTainting;
        done?: boolean;
        timingAllowFailedFlag?: boolean;
    };

    type InnerResponse = {
        type: ResponseType;
        aborted?: boolean;
        url: URL | null;
        urlList: URL[];
        status: number;
        statusMessage: string;
        headerList: [string, string][];
        body: Uint8Array | ExtractedBody | null;
        cacheState: "" | "local" | "validated";
    };

    interface ExtractedBody {
        stream: ReadableStream;
        source: any;
        length: number | null;
        type: string | null;
    }

    class Request {
        constructor(input: RequestInfo, init?: RequestInit);
        get method(): string;
        get url(): string;
        get headers(): Headers;
        get destination(): RequestDestination;
        get referrer(): string;
        get referrerPolicy(): ReferrerPolicy;
        get mode(): RequestMode;
        get credentials(): RequestCredentials;
        get cache(): RequestCache;
        get redirect(): RequestRedirect;
        get integrity(): string;
        get keepalive(): boolean;
        get signal(): AbortSignal;
        get duplex(): string;
        clone(): Request;
    }

    class Response {
        constructor(body?: BodyInit | null, init?: ResponseInit);
        static json(data: any, init?: ResponseInit): Response;
        static error(): Response;
        static redirect(url: string, status: number): Response;
        get type(): ResponseType;
        get url(): string;
        get redirected(): boolean;
        get status(): number;
        get ok(): boolean;
        get statusText(): string;
        get headers(): Headers;
        clone(): Response;
    }

    function fetch(input: RequestInfo, init?: RequestInit): Promise<Response>;

    function serve(options: ServeOptions | ServerHandler | (ServeOptions & TlsCertificate), _serverOptions?: ServeOptions | (ServeOptions & TlsCertificate)): void;

    export {
        AbortController,
        AbortSignal,
        createDependentAbortSignal,
        DOMException,
        emptyHeader,
        fetch,
        fillHeadersMapFrom,
        headerInnerList,
        Headers,
        Request,
        Response,
        serve,
        TextDecoder,
        TextEncoder,
        URL,
        URLSearchParams
    };
}
