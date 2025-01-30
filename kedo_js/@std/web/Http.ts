import { asyncOp } from "@kedo/utils";
import {
  isInReadableState,
  ReadableStream,
  readableStreamResource
} from "@kedo:int/std/stream";
import {
  InternalSignal,
  op_http_request_body,
  op_http_request_headers,
  op_http_request_keep_alive,
  op_http_request_method,
  op_http_request_redirect_count,
  op_http_request_uri,
  op_internal_fetch,
  op_internal_start_server,
  op_new_fetch_client,
  op_read_decoded_stream,
  op_send_event_response,
  op_send_signal
} from "@kedo:op/web";
import { AbortSignal, createDependentAbortSignal, newAbortSignal } from "./AbortSignal";
import { extractBody, mixinBody } from "./Body";
import { emptyHeader, fillHeadersMapFrom, headerInnerList, Headers, headersFromInnerList } from "./Headers";

type RequestInfo = Request | string;

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
const _internalHeaders = Symbol("[internalHeaders]");
const _illegalConstructor = Symbol("[illegalConstructor]");
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

function isExtractedBody(body: any): body is ExtractedBody {
  return !(body instanceof Uint8Array);
}

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

class Request {
  [_request]!: InnerRequest;
  [_headers]?: Headers;
  [_signal]!: AbortSignal;

  constructor(input: RequestInfo, init?: RequestInit) {
    if ((input as any) === _illegalConstructor) return;
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
      signal = init.signal as AbortSignal;
    }

    request.priority = init?.priority ?? request.priority;
    this[_request] = request;
    if (signal) {
      this[_signal] = createDependentAbortSignal([signal]);
    } else {
      this[_signal] = newAbortSignal();
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
    mixinBody(this, _requestBody as any);
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

  get [_internalHeaders](): Headers {
    if (this[_headers] === undefined) {
      this[_headers] = headersFromInnerList(this[_request].header_list);
    }

    return this[_headers];
  }

  get headers(): Headers {
    return this[_internalHeaders];
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


const createResponse = (
  response: InnerResponse,
  guard: HeadersGuard,
): Response => {
  const res = new Response(_illegalConstructor as any);
  res[_response] = response;
  res[_headers] = new Headers([]);
  fillHeadersMapFrom(response.headerList, res[_headers], guard);
  mixinBody(res, _responseBody as any);
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
    mixinBody(this, _responseBody as any);
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
    mixinBody(response, _responseBody as any);
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

const client = op_new_fetch_client();

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
  const fetchRequest: HttpRequest = {
    method: request.method,
    url: request.url.toString(),
    header_list: request.header_list,
    signal: internalSignal,
    redirect: REDIRECT_MAP[request.redirect],
  };
  if (isExtractedBody(request.body)) {
    if (request.body.source !== null) {
      fetchRequest.source = request.body.source;
    } else if (request.body.stream !== null) {
      fetchRequest.stream = readableStreamResource(request.body.stream);
    }
  }

  const fetchResponse = await asyncOp(op_internal_fetch, client, fetchRequest);

  let responseBody: ExtractedBody | null = null;
  if (fetchResponse.body) {
    const response_type = fetchResponse.headers.find(
      ([name]) => name === "content-type",
    )?.[1];
    const stream_body = decodedStreamToReadableStream(fetchResponse.body);
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

function decodedStreamToReadableStream(
  responseStream: DecodedBodyStream,
): ReadableStream {
  // instantiate a new ReadableStream object wiht the pull function to read from the response stream to a ReadableStream
  return new ReadableStream({
    type: "bytes",
    async pull(controller) {
      const chunk = await asyncOp(op_read_decoded_stream, responseStream);
      if (chunk) {
        controller.enqueue(chunk);
      } else {
        controller.close();
      }
    },
  });
}

// ------------------------------------------------------------
//                         Server                              |
// ------------------------------------------------------------
type ServeOptions = {
  hostname?: string;
  port?: number;
  signal?: AbortSignal;
  onListen?: (args: any) => void;
  handler?: ServerHandler;
};

type ServerHandler = (request: Request) => Promise<Response>;

function internalServerError(): HttpResponse {
  return {
    status: 500,
    url: "/",
    status_message: "Internal Server Error",
    headers: [],
  };
}

function serve(
  options: ServeOptions | ServerHandler | (ServeOptions & TlsCertificate),
  _serverOptions?: ServeOptions | (ServeOptions & TlsCertificate),
): void {
  let handler: ServerHandler | undefined;
  let tlsCertificate: TlsCertificate | undefined;
  let serverOptions: ServeOptions | undefined;
  let internalSignal = new InternalSignal();

  if (typeof options === "function") {
    handler = options;
    serverOptions = _serverOptions as ServeOptions;
    tlsCertificate = _serverOptions as TlsCertificate;
  } else if (typeof options === "object") {
    if (typeof options.handler === "function") {
      handler = options.handler;
      serverOptions = options;
      tlsCertificate = _serverOptions as TlsCertificate;
    }
  }

  if (serverOptions?.signal && serverOptions.signal instanceof AbortSignal) {
    serverOptions.signal.addEventListener("abort", () => {
      op_send_signal(internalSignal);
    });
  }

  if (handler === undefined) {
    throw new TypeError("Handler is required");
  }

  // prepare the internal options
  let internalOptions: InternalServerOptions = {
    hostname: serverOptions?.hostname || "0.0.0.0",
    port: serverOptions?.port || 8080,
    key: tlsCertificate?.key,
    cert: tlsCertificate?.cert,
    handler: serverHandler(handler),
    signal: internalSignal,
  };

  asyncOp(op_internal_start_server, internalOptions)
    .then((address) => {
      const [hostname, port] = address.split(":");
      serverOptions?.onListen?.({
        hostname: hostname || internalOptions.hostname,
        port: port || internalOptions.port,
        key: internalOptions.key,
        cert: internalOptions.cert,
      });
    })
    .catch((error) => {
      throw error;
    });
}

function serverHandler(handler: ServerHandler): InternalServerHandler {
  const asyncHandler = async (request: Request) => {
    const response = await handler(request);
    return response;
  };

  const internalHandler = (request: HttpRequestResource, sender: RequestEventResource) => {
    const requestObject = _requestFromHttpRequest(request);
    asyncHandler(requestObject)
      .then((response) => {
        let headersList: [string, string][] = [];
        if (response.headers instanceof Headers) {
          headersList = Array.from(response.headers.entries());
        }

        let innerResponse = response[_response] as InnerResponse;
        let responseBody: any = undefined;
        if (isExtractedBody(innerResponse.body)) {
          if (innerResponse.body.source) {
            responseBody = { source: innerResponse.body.source };
          } else if (innerResponse.body.stream) {
            responseBody = { stream: readableStreamResource(innerResponse.body.stream) };
          }
        }

        const httpResponse: HttpResponse = {
          url: requestObject.url,
          status: innerResponse.status,
          status_message: innerResponse.statusMessage,
          headers: headersList,
          ...responseBody,
        };

        op_send_event_response(sender, httpResponse);
      })
      .catch((error) => {
        console.log("Error: ", error.message);
        op_send_event_response(sender, internalServerError());
      });
  }

  return internalHandler;
}

class InnerRequestResource {
  #method?: string;
  #url?: string;
  #headerList?: [string, string][];
  #keepalive?: boolean;
  #redirectCount?: number;
  #body?: Uint8Array | ExtractedBody | null;
  private requestResource: HttpRequestResource;

  constructor(innerRequest: HttpRequestResource) {
    this.requestResource = innerRequest;
  }

  get method() {
    if (!this.#method) {
      this.#method = op_http_request_method(this.requestResource);
    }

    return this.#method;
  }

  get url() {
    if (!this.#url) {
      this.#url = op_http_request_uri(this.requestResource);
    }

    return this.#url;
  }

  get headerList() {
    // return [];
    if (!this.#headerList) {
      this.#headerList = op_http_request_headers(this.requestResource);
    }

    return this.#headerList;
  }

  get keepalive() {
    if (this.#keepalive === undefined) {
      this.#keepalive = op_http_request_keep_alive(this.requestResource);
    }

    return this.#keepalive;
  }

  get redirectCount() {
    if (this.#redirectCount === undefined) {
      this.#redirectCount = op_http_request_redirect_count(this.requestResource);
    }

    return this.#redirectCount;
  }

  get body(): Uint8Array | ExtractedBody | null {
    if (this.#body === undefined) {
      const requestBody = op_http_request_body(this.requestResource);
      if (requestBody?.source) {
        this.#body = requestBody.source;
      } else if (requestBody?.stream) {
        const stream = decodedStreamToReadableStream(requestBody.stream);
        const bodyWithType = extractBody(stream, this.keepalive);
        this.#body = bodyWithType.body;
      } else {
        this.#body = null;
      }
    }

    return this.#body as Uint8Array | ExtractedBody | null;
  }
}

// create internal inner request
const createInnerRequestFromResource = (innerRequest: HttpRequestResource): InnerRequest => {
  const request = new InnerRequestResource(innerRequest);
  return {
    get method() { return request.method },
    get url() { return new URL(request.url) },
    get header_list() { return request.headerList },
    get keepalive() { return request.keepalive },
    get redirectCount() { return request.redirectCount },
    get body() { return request.body },
    unsafeRequestFlag: false,
    get urlList() { return [this.url] },
    get currentURL() { return this.url },
    initiatorType: "fetch",
    mode: "cors",
    credentials: "same-origin",
    cache: "default",
    redirect: "follow",
    referrer: "client",
    origin: "client",
    responseTainting: "basic",
    referrerPolicy: "",
    integrity: "",
    priority: "auto"
  };
}

function _requestFromHttpRequest(httpRequest: HttpRequestResource): Request {
  const innerRequest = createInnerRequestFromResource(httpRequest);
  const request = new Request(_illegalConstructor as any);
  request[_request] = innerRequest;
  mixinBody(request, _requestBody as any);
  return request;
}

export { fetch, Request, Response, serve };
