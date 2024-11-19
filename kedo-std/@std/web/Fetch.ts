import {
  InternalSignal,
  op_internal_fetch,
  op_read_response_stream,
  op_send_signal,
} from "@kedo/internal/utils";
import {
  isDisturbed,
  isErrored,
  isInReadableState,
  ReadableStream,
  readableStreamCloseByteController,
  readableStreamEnqueue,
  readableStreamResource,
} from "@kedo/stream";
import {
  AbortSignal,
  createDependentAbortSignal,
  emptyHeader,
  fillHeadersMapFrom,
  headerInnerList,
  Headers,
} from "@kedo/web/internals";
// import { Headers, fillHeadersMapFrom, emptyHeader } from "./Headers";
// import { AbortSignal, createDependentAbortSignal } from "./AbortSignal";
import { assert } from "../utils";

type RequestInfo = Request | string;
type ResponseTainting = "basic" | "cors" | "opaque";

type InnerRequest = {
  method: string;
  url: URL;
  localURLsOnlyFlag?: boolean;
  header_list: [string, string][];
  unsafeRequestFlag?: boolean;
  // byte sequence or extracted body or null
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

type ResponseType =
  | "basic"
  | "cors"
  | "default"
  | "error"
  | "opaque"
  | "opaqueredirect";

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

type ResponseInit = {
  status?: number;
  statusText?: string;
  headers?: Headers | [string, string][] | Record<string, string>;
};

const _request = Symbol("[request]");
const _response = Symbol("[response]");
const _requestBody = Symbol("[requestBody]");
const _responseBody = Symbol("[responseBody]");
const _headers = Symbol("[headers]");
const _signal = Symbol("[signal]");

const HTTP_METHODS = {
  DELETE: "DELETE",
  POST: "POST",
  GET: "GET",
  PUT: "PUT",
  OPTIONS: "OPTIONS",
  PATCH: "PATCH",
  HEAD: "HEAD",
  delete: "DELETE",
  get: "GET",
  options: "OPTIONS",
  patch: "PATCH",
  head: "HEAD",
  post: "POST",
  put: "PUT",
};

// https://fetch.spec.whatwg.org/#request-class
// |----------------------------------------------------------|
// |                        Request                           |
// |----------------------------------------------------------|
const createInnerRequest = (parsedURL: URL): InnerRequest => {
  return {
    method: "GET",
    url: parsedURL,
    header_list: [],
    body: null,
    keepalive: false,
    priority: "auto",
    origin: "client",
    referrer: "client",
    referrerPolicy: "",
    mode: "no-cors",
    redirect: "follow",
    cache: "default",
    initiatorType: "fetch",
    integrity: "",
    credentials: "same-origin",
    urlList: [new URL(parsedURL.href)],
    get currentURL() {
      return this.urlList[this.urlList.length - 1];
    },
    redirectCount: 0,
    responseTainting: "basic",
  };
};

interface ExtractedBody {
  stream: ReadableStream;
  source: any;
  length: number | null;
  type: string | null;
}

type BodyInit =
  | Blob
  | BufferSource
  | FormData
  | URLSearchParams
  | ReadableStream
  | string;

const isReadableStream = (object: any): object is ReadableStream =>
  object instanceof ReadableStream;

const unusable = (stream: ReadableStream | null) => {
  if (stream === null) return false;
  return isDisturbed(stream as any) || stream.locked;
};

class InternalBody {
  private _body: ReadableStream | null;
  private _bodyUsed: boolean;
  private _headers: Headers;

  constructor(body: ReadableStream | null, headers: Headers) {
    this._body = body;
    this._bodyUsed = false;
    this._headers = headers;
  }

  get body(): ReadableStream | null {
    return this._body;
  }

  get bodyUsed(): boolean {
    return this._body !== null && isDisturbed(this._body as any);
  }

  // Consume Body:
  // The Consume body function consist of converting the byte sequence into javascrip value
  // - 1. Check wheter the body is unsable by checking if it is different from null and stream is no disturbed or locked
  // - 2. If body is null, then return null
  // - 3. Fully Read the body:
  //     - 3.1. Start a parrallel bytes queue
  //     - 3.2. let reader be the result of acquiring a reader from body's stream
  //     - 3.3. read all the bytes from the reader and add them to the queue
  // - 4. resolve the prmise with the result of converting the queue into a javascript value
  private async consumeBody(): Promise<Uint8Array> {
    if (this.bodyUsed) {
      throw new TypeError("Body has already been consumed.");
    }

    if (this._body === null) {
      return new Uint8Array();
    }

    // 1. If object is unusable, then return a promise rejected with a TypeError.
    if (unusable(this._body)) {
      throw new TypeError("Body is unusable");
    }

    const reader = this._body.getReader<ReadableStreamDefaultReader>();
    const chunks: Uint8Array[] = [];
    let done: boolean | undefined = false;

    while (!done) {
      // Allocate a new buffer (e.g., 1KB) for each read
      // TODO: Use a more efficient way to read the stream
      // const buffer = new Uint8Array(1024);
      const { value, done: readerDone } = await reader.read();
      if (value && value.byteLength > 0) {
        chunks.push(value);
      }
      done = readerDone;
    }

    this._bodyUsed = true;

    // Combine all chunks into a single Uint8Array
    const totalLength = chunks.reduce(
      (sum, chunk) => sum + chunk.byteLength,
      0,
    );
    const result = new Uint8Array(totalLength);
    let offset = 0;

    for (const chunk of chunks) {
      result.set(chunk, offset);
      offset += chunk.byteLength;
    }

    return result;
  }

  async arrayBuffer(): Promise<ArrayBuffer> {
    const bytes = await this.consumeBody();
    return bytes.buffer.slice(
      bytes.byteOffset,
      bytes.byteOffset + bytes.byteLength,
    ) as ArrayBuffer;
  }

  async bytes(): Promise<Uint8Array> {
    return this.consumeBody();
  }

  async json(): Promise<any> {
    const text = await this.text();
    try {
      return JSON.parse(text);
    } catch (e) {
      throw new SyntaxError("Failed to parse JSON.");
    }
  }

  async text(): Promise<string> {
    const bytes = await this.consumeBody();
    return new TextDecoder("utf-8").decode(bytes);
  }

  getMimeType(): string | null {
    const contentType = this._headers.get("content-type");
    return contentType;
  }
}

// TODO: this implementation must be optimized
function extractBody(
  object: BodyInit,
  keepalive = false,
): { body: ExtractedBody; type: string | null } {
  // Let stream be null.
  let stream: ReadableStream | null = null;
  let source: Uint8Array | null = null;
  let length: number | null = null;
  let type: string | null = null;

  if (isReadableStream(object)) {
    if (keepalive) {
      throw new TypeError(
        "ReadableStream cannot be used with keepalive set to true.",
      );
    }

    if (isDisturbed(object as any) || object.locked) {
      throw new TypeError("ReadableStream is unusable.");
    }

    stream = object;
  } else {
    stream = new ReadableStream({ type: "bytes" });
  }

  assert(stream instanceof ReadableStream);

  if (object instanceof ArrayBuffer || ArrayBuffer.isView(object)) {
    // Byte sequence or BufferSource
    if (object instanceof ArrayBuffer) {
      source = new Uint8Array(object).slice();
    } else {
      source = new Uint8Array(
        object.buffer,
        object.byteOffset,
        object.byteLength,
      ).slice();
    }
  } else if (object instanceof URLSearchParams) {
    // URLSearchParams
    source = new TextEncoder().encode(object.toString());
    type = "application/x-www-form-urlencoded;charset=UTF-8";
  } else if (typeof object === "string") {
    // Scalar value string
    source = new TextEncoder().encode(object);
    type = "text/plain;charset=UTF-8";
  } else if (!isReadableStream(object)) {
    throw new TypeError("Invalid body type");
  }

  if (ArrayBuffer.isView(source)) {
    length = source.byteLength;
    if (!isErrored(stream as any)) {
      readableStreamEnqueue(stream as any, source);
      readableStreamCloseByteController(stream as any);
    }
  }

  const body: ExtractedBody = { stream, source, length, type };
  return { body, type };
}

type MixingInput = Request | Response;

const mixinBody = (input: MixingInput, _bodyKey: symbol) => {
  const body = input[_bodyKey] as ExtractedBody | null;
  const innerBody = new InternalBody(body?.stream || null, input[_headers]);

  const mixin = {
    body: {
      get(): ReadableStream | null {
        return innerBody.body;
      },
      configurable: true,
      enumerable: true,
    },
    bodyUsed: {
      get(): boolean {
        return innerBody.bodyUsed;
      },
    },
    arrayBuffer: {
      value: function arrayBuffer(): Promise<ArrayBuffer> {
        return innerBody.arrayBuffer();
      },
      writable: true,
      configurable: true,
      enumerable: true,
    },
    bytes: {
      value: function bytes(): Promise<Uint8Array> {
        return innerBody.bytes();
      },
      writable: true,
      configurable: true,
      enumerable: true,
    },
    json: {
      value: function json(): Promise<any> {
        return innerBody.json();
      },
      writable: true,
      configurable: true,
      enumerable: true,
    },
    text: {
      value: function text(): Promise<string> {
        return innerBody.text();
      },
      writable: true,
      configurable: true,
      enumerable: true,
    },
  };

  Object.defineProperties(input, mixin);
};

class Request {
  [_request]: InnerRequest;
  [_headers]: Headers;
  [_signal]: AbortSignal;

  constructor(input: RequestInfo, init?: RequestInit) {
    // 1. Let request be null.
    let request: InnerRequest | null = null;
    // 2. Let fallbackMode be null.
    let fallbackMode: RequestMode | null = null;
    // 3. Let signal be null.
    let signal: AbortSignal | null = null;
    if (typeof input === "string") {
      // 4. If input is a string, then:
      // 4.1. Let parsedURL be the result of parsing input with baseURL.
      const parsedURL = new URL(input);
      // 4.2. Set request to parsedURL.
      request = createInnerRequest(parsedURL);
      // 4.3. Set fallbackMode to "cors".
      fallbackMode = "cors";
    } else {
      // 5.2. Set request to input’s request.
      request = input[_request] as InnerRequest;
      signal = input[_signal] as AbortSignal;
    }

    // TODO: ORIGIN
    // let origin = request.url.origin;
    request = {
      ...request,
      header_list: [...request.header_list],
      unsafeRequestFlag: false,
      urlList: [...request.urlList],
      initiatorType: "fetch",
    };

    // 13. If init is not empty, then:
    if (init !== undefined) {
      // 13.1 If request’s mode is "navigate", then set it to "same-origin".
      if (request.mode === "navigate") {
        request.mode = "same-origin";
      }

      request.origin = "client";
      request.referrer = "client";
      request.referrerPolicy = "";
      request.url = request.currentURL;
      request.urlList = [request.url];
    }

    // 14. If init["referrer"] exists, then:
    if (init?.referrer !== undefined) {
      // TODO: revisit this implementation
      let referrer = init.referrer;
      if (referrer === "") {
        referrer = "no-referrer";
      } else {
        request.referrer = referrer;
      }
    }

    request.referrerPolicy = init?.referrerPolicy ?? request.referrerPolicy;

    let mode = init?.mode ?? fallbackMode ?? "cors";
    if (request.mode === "navigate") throw new TypeError("Invalid mode.");
    request.mode = mode;

    request.credentials = init?.credentials ?? request.credentials;
    request.cache = init?.cache ?? request.cache;
    if (request.cache === "only-if-cached" && request.mode !== "same-origin") {
      throw new TypeError("Invalid cache mode.");
    }

    request.redirect = init?.redirect ?? request.redirect;
    request.integrity = init?.integrity ?? request.integrity;
    if (init?.keepalive !== undefined) {
      request.keepalive = init.keepalive;
    }

    if (init?.method !== undefined) {
      let method = init.method;
      if (!HTTP_METHODS[method as keyof typeof HTTP_METHODS]) {
        throw new TypeError("Invalid method.");
      }
      request.method = HTTP_METHODS[method as keyof typeof HTTP_METHODS];
    }

    if (init?.signal) {
      signal = init.signal as any as AbortSignal;
    }

    request.priority = init?.priority ?? request.priority;
    this[_request] = request;
    if (signal) {
      this[_signal] = createDependentAbortSignal([signal]);
    }

    this[_headers] = new Headers(request.header_list);
    if (init?.headers) {
      const headers = init.headers;
      emptyHeader(this[_headers]);
      fillHeadersMapFrom(headers as any, this[_headers], "request");
      request.header_list = headerInnerList(this[_headers]);
    }

    let inputBody = null;
    if (input instanceof Request) {
      inputBody = input[_request].body;
    }

    // 35. If either init["body"] exists and is non-null or inputBody is non-null, and request’s method is `GET` or `HEAD`, then throw a TypeError.
    if (
      (request.method === "GET" || request.method === "HEAD") &&
      ((init?.body !== undefined && init.body !== null) || inputBody !== null)
    ) {
      throw new TypeError("A GET/HEAD request cannot have a body.");
    }

    // 36. Let initBody be null.
    let initBody: ExtractedBody | null = null;
    // 37. If init["body"] exists and is non-null, then:
    if (init?.body !== undefined && init.body !== null) {
      // 37.1. Let bodyWithType be the result of extracting init["body"], with keepalive set to request’s keepalive.
      const bodyWithType = extractBody(init.body, request.keepalive);
      // 37.2. Set initBody to bodyWithType.body.
      initBody = bodyWithType.body as any;
      // 37.3. Let type be bodyWithType’s type.
      const type = bodyWithType.type;
      // 37.4. If type is non-null and this’s headers’s header list does not contain `Content-Type`, then append (`Content-Type`, type) to this’s headers.
      if (type !== null && !this[_headers].has("Content-Type")) {
        this[_headers].append("Content-Type", type);
      }
    }

    // 38. Let inputOrInitBody be initBody if it is non-null; otherwise inputBody.
    const inputOrInitBody = initBody ?? inputBody;
    // 39. If inputOrInitBody is non-null and inputOrInitBody’s source is null, then:
    if (inputOrInitBody !== null && (inputOrInitBody as any).source === null) {
      // 39.1. If initBody is non-null and init["duplex"] does not exist, then throw a TypeError.
      // if (initBody !== null && init?.duplex === undefined) {
      //   throw new TypeError("Body is already used.");
      // }
      // 39.2. If this’s request’s mode is neither "same-origin" nor "cors", then throw a TypeError.
      if (request.mode !== "same-origin" && request.mode !== "cors") {
        throw new TypeError("Invalid mode.");
      }
      // 39.3. Set this’s request’s use-CORS-preflight flag.
      request.useCORSPreflightFlag = true;
    }
    // 40. Let finalBody be inputOrInitBody.
    // let finalBody = inputOrInitBody;
    // 42. Set this’s request’s body to finalBody.
    this[_request].body = inputOrInitBody;
    mixinBody(this, _requestBody);
  }

  get [_requestBody](): Uint8Array | ExtractedBody | null {
    return this[_request]?.body;
  }

  get method(): string {
    return this[_request].method;
  }

  get url(): string {
    return this[_request].url.toString();
  }

  get headers(): Headers {
    return this[_headers];
  }

  get destination(): RequestDestination {
    return ""; // TODO
  }

  get referrer(): string {
    return this[_request].referrer;
  }

  get referrerPolicy(): ReferrerPolicy {
    return this[_request].referrerPolicy;
  }

  get mode(): RequestMode {
    return this[_request].mode;
  }

  get credentials(): RequestCredentials {
    return this[_request].credentials;
  }

  get cache(): RequestCache {
    return this[_request].cache;
  }

  get redirect(): RequestRedirect {
    return this[_request].redirect;
  }

  get integrity(): string {
    return this[_request].integrity;
  }

  get keepalive(): boolean {
    return this[_request].keepalive;
  }

  get signal(): AbortSignal {
    return this[_signal];
  }

  get duplex(): string {
    return "half";
  }

  // TODO: Implement the clone method after supporting the tee method for ReadableStream
  // https://fetch.spec.whatwg.org/#ref-for-dom-request-clone
  clone(): Request {
    throw new Error("Method not implemented.");
  }
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

// https://fetch.spec.whatwg.org/#response-class
// |----------------------------------------------------------|
// |                        Response                          |
// |----------------------------------------------------------|

const createInnerResponse = (): InnerResponse => {
  const response: InnerResponse = {
    type: "default",
    get url() {
      if (this.urlList.length === 0) return null;
      return this.urlList[this.urlList.length - 1];
    },
    urlList: [],
    status: 200,
    statusMessage: "",
    headerList: [],
    body: null,
    cacheState: "",
    aborted: false,
  };
  return response;
};

const reasonPhraseRegex = /^[\t !-~\x80-\xFF]+$/;

const isReasonPhraseToken = (input: string): boolean => {
  return reasonPhraseRegex.test(input);
};

const isNullBodyStatus = (status: number): boolean => {
  return status === 101 || status === 204 || status === 205 || status === 304;
};

const isRedirectStatus = (status: number): boolean => {
  return (
    status === 301 ||
    status === 302 ||
    status === 303 ||
    status === 307 ||
    status === 308
  );
};

const initializeResponse = (
  response: Response,
  init?: ResponseInit,
  bodyWithType?: {
    body: ExtractedBody;
    type: string | null;
  },
): void => {
  // 1. If init["status"] is not in the range 200 to 599, inclusive, then throw a RangeError.
  if (
    init?.status !== undefined &&
    (init.status < 200 || init.status > 599) &&
    init.status !== 101
  ) {
    throw new RangeError("Status code is out of range.");
  }
  // 2. If init["statusText"] does not match the reason-phrase token production, then throw a TypeError.
  if (init?.statusText !== undefined && !isReasonPhraseToken(init.statusText)) {
    throw new TypeError("Invalid status text.");
  }
  // 3. Set response’s response’s status to init["status"].
  response[_response].status = init?.status ?? 200;
  // 4. Set response’s response’s status message to init["statusText"].
  response[_response].statusMessage = init?.statusText ?? "";
  // 5. If init["headers"] exists, then fill response’s headers with init["headers"].
  if (init?.headers !== undefined) {
    fillHeadersMapFrom(init.headers, response[_headers], "response");
  }
  // 6. If bodyWithType is non-null, then:
  if (bodyWithType !== undefined) {
    // If response’s status is a null body status, then throw a TypeError.
    if (isNullBodyStatus(response[_response].status)) {
      throw new TypeError("Status code does not allow body.");
    }

    const { body, type } = bodyWithType;
    // Set response’s response’s body to body.
    response[_response].body = body;
    // If body’s type is non-null and response’s header list does not contain `Content-Type`, then append (`Content-Type`, body’s type) to response’s header list.
    if (
      type !== null &&
      !response[_response].headerList.some(
        ([name]) => name?.toLowerCase() === "content-type",
      )
    ) {
      response[_response].headerList.push(["Content-Type", type]);
    }
  }
};

const _illegalConstructor = Symbol("[illegalConstructor]");

const createResponse = (
  response: InnerResponse,
  guard: HeadersGuard,
): Response => {
  const res = new Response(_illegalConstructor as any);
  res[_response] = response;
  res[_headers] = new Headers([]);
  fillHeadersMapFrom(response.headerList, res[_headers], guard);
  mixinBody(res, _responseBody);
  return res;
};

class Response {
  [_headers]: Headers;
  [_response]: InnerResponse;

  constructor(body?: BodyInit | null, init?: ResponseInit) {
    if ((body as any) === _illegalConstructor) return;

    // 1. Let response be a new response.
    this[_response] = createInnerResponse();
    // 2. Set this’s headers to a new Headers object with this’s relevant realm, whose header list is this’s response’s header list and guard is "response".
    this[_headers] = new Headers(this[_response].headerList);
    // fillHeadersMapFrom(init?.headers, this[_headers], response.header_list);
    const bodyWithType = body && body !== null ? extractBody(body) : undefined;
    // 3. Perform initialize a response given this, init, and bodyWithType.
    initializeResponse(this, init, bodyWithType);
    mixinBody(this, _responseBody);
  }

  get [_responseBody](): Uint8Array | ExtractedBody | null {
    return this[_response].body;
  }

  static json(data: any, init?: ResponseInit): Response {
    const bodyWithType = extractBody(JSON.stringify(data));
    bodyWithType.type = "application/json";

    const response = new Response(_illegalConstructor as any, init);
    response[_response] = createInnerResponse();
    response[_headers] = new Headers(response[_response].headerList);
    initializeResponse(response, init, bodyWithType);
    mixinBody(response, _responseBody);
    return response;
  }

  static error(): Response {
    const response = new Response(_illegalConstructor as any);
    response[_response] = createInnerResponse();
    response[_headers] = new Headers([]);
    fillHeadersMapFrom(
      response[_response].headerList,
      response[_headers],
      "immutable",
    );
    response[_response].type = "error";
    response[_response].status = 0;
    return response;
  }

  static redirect(url: string, status: number): Response {
    // Let parsedURL be the result of parsing url with current settings object’s API base URL.
    const parsedURL = new URL(url);
    // If status is not a redirect status, then throw a RangeError.
    if (!isRedirectStatus(status)) {
      throw new RangeError("Invalid redirect status.");
    }
    // Let responseObject be the result of creating a Response object, given a new response, "immutable", and the current realm.
    const response = new Response(_illegalConstructor as any);
    response[_response] = createInnerResponse();
    // Set responseObject’s response’s status to status.
    response[_response].status = status;
    // Let value be parsedURL, serialized and isomorphic encoded.
    const value = parsedURL.href;
    // Append (`Location`, value) to responseObject’s response’s header list.
    response[_response].headerList.push(["Location", value]);
    response[_headers] = new Headers(response[_response].headerList);
    // Return responseObject.
    return response;
  }

  get type(): ResponseType {
    return this[_response].type;
  }

  get url(): string {
    const url = this[_response].url;
    if (url === null) return "";

    const parsedUrl = new URL(url);
    parsedUrl.hash = "";
    return parsedUrl.href;
  }

  get redirected(): boolean {
    return this[_response].urlList.length > 1;
  }

  get status(): number {
    return this[_response].status;
  }

  get ok(): boolean {
    return this[_response].status >= 200 && this[_response].status < 300;
  }

  get statusText(): string {
    return this[_response].statusMessage;
  }

  get headers(): Headers {
    return this[_headers];
  }

  // TODO: Implement this method
  clone(): Response {
    throw new Error("Not implemented.");
  }
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

const REDIRECT_MAP = {
  follow: 0,
  error: 1,
  manual: 2,
};

// https://fetch.spec.whatwg.org/#fetch-method
// |------------------------------------------------------------|
// |                        FETCH                               |
// |------------------------------------------------------------|
function fetch(input: RequestInfo, init?: RequestInit): Promise<Response> {
  return new Promise<Response>((resolve, reject) => {
    // 1. Create the Request object
    const requestObject = new Request(input, init);
    const request = requestObject[_request];

    // 2. Handle the abort signal
    if (requestObject.signal && requestObject.signal.aborted) {
      // const error = new Error("The operation was aborted.") as any;
      // error.name = "AbortError";
      abortFetch(reject, request as any, null, requestObject.signal.reason);
      return;
    }

    if (!requestObject.headers.has("Accept")) {
      request.header_list.push(["Accept", "*/*"]);
    }

    if (!requestObject.headers.has("Accept-Language")) {
      request.header_list.push(["Accept-Language", "*"]);
    }

    if (!requestObject.headers.has("User-Agent")) {
      request.header_list.push(["User-Agent", "Kedo/1.0"]);
    }

    if (!requestObject.headers.has("Accept-Encoding")) {
      request.header_list.push(["Accept-Encoding", "gzip, deflate, zstd, br"]);
    }

    // 3. Set up for aborting if signal is later triggered
    let locallyAborted = false;
    let controller: AbortController | null = new AbortController();
    let internalSignal = new InternalSignal();

    function onAbort() {
      locallyAborted = true;
      if (controller) {
        controller.abort(requestObject.signal.reason);
      }

      op_send_signal(internalSignal);
      abortFetch(reject, request as any, null, requestObject.signal.reason);
    }

    try {
      requestObject.signal?.addEventListener("abort", onAbort);

      // 4. Begin the fetch process
      const processResponse = (response: InnerResponse) => {
        if (locallyAborted) {
          return;
        }

        if (response.aborted) {
          const deserializedError = controller.signal.reason;
          // deserializedError.name = "AbortError";
          abortFetch(reject, request as any, null, deserializedError);
          return;
        }

        if (response.status === 0 && response.type === "error") {
          reject(new TypeError("Network error"));
          return;
        }

        const responseObject = createResponse(response, "immutable");
        resolve(responseObject);
      };

      performFetch(request, internalSignal)
        .then(processResponse)
        .catch(reject)
        .finally(() => {
          requestObject.signal?.removeEventListener("abort", onAbort);
        });
    } catch (error) {
      reject(error);
      requestObject.signal?.removeEventListener("abort", onAbort);
    }
  });
}

// Helper function to handle aborting the fetch process
function abortFetch(
  reject: (reason?: any) => void,
  request: IRequest,
  response: IResponse | null,
  error: any,
) {
  reject(error);

  // If request body is readable, cancel it
  if (request.body) {
    request.body.cancel(error).catch((error) => { });
  }

  if (response === null) return;

  // If response body is readable, error it
  if (response.body && isInReadableState(response.body)) {
    response.body.cancel(error).catch((error) => { });
  }
}

// Stub for the actual network fetch operation
async function performFetch(
  request: InnerRequest,
  internalSignal?: InternalSignal,
): Promise<InnerResponse> {
  const fetchRequest: FetchRequest = {
    method: request.method,
    url: request.url.toString(),
    header_list: request.header_list,
    signal: internalSignal,
    redirect: REDIRECT_MAP[request.redirect],
  }
  // let stream: ReadableStreamResource | undefined;
  if (request.body && (request.body as ExtractedBody).stream) {
    fetchRequest.stream = readableStreamResource(
      (request.body as ExtractedBody).stream,
    );
  } else if (request.body && (request.body as ExtractedBody).source) {
    fetchRequest.source = (request.body as ExtractedBody).source;
  }

  const fetchResponse = await op_internal_fetch(fetchRequest);

  let responseBody: ExtractedBody | null = null;
  if (fetchResponse.body) {
    const response_type = fetchResponse.headers.find(
      ([name]) => name === "content-type",
    )?.[1];
    const stream_body = fetchResponseStreamToReadableStream(fetchResponse.body);
    responseBody = {
      stream: stream_body,
      source: null,
      length: null,
      type: response_type || null,
    };
  }

  const response: InnerResponse = {
    type: "default",
    get url() {
      if (this.urlList.length === 0) return null;
      return this.urlList[this.urlList.length - 1];
    },
    urlList: request.urlList,
    status: fetchResponse.status,
    statusMessage: fetchResponse.status_message,
    headerList: fetchResponse.headers,
    body: responseBody,
    cacheState: "",
  };
  return response;
}

function fetchResponseStreamToReadableStream(
  responseStream: ResponseStream,
): ReadableStream {
  // instantiate a new ReadableStream object wiht the pull function to read from the response stream to a ReadableStream
  return new ReadableStream({
    type: "bytes",
    async pull(controller) {
      const chunk = await op_read_response_stream(responseStream);
      if (chunk) {
        controller.enqueue(chunk);
      } else {
        controller.close();
      }
    },
  });
}

export { fetch, Request, Response };
