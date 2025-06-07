import { assert } from "@kedo/utils";
import {
    op_http_request_body,
    op_http_request_headers,
    op_http_request_keep_alive,
    op_http_request_method,
    op_http_request_redirect_count,
    op_http_request_uri,
} from "@kedo:op/web";
import {
    AbortSignal,
    createDependentAbortSignal,
    newAbortSignal,
} from "./AbortSignal";
import { extractBody, mixinBody } from "./Body";
import {
    _headers,
    _illegalConstructor,
    _internalHeaders,
    _request,
    _requestBody,
    _signal,
    decodedStreamToReadableStream,
    HTTP_METHODS,
} from "./commons";
import {
    emptyHeader,
    fillHeadersMapFrom,
    headerInnerList,
    Headers,
    headersFromInnerList,
} from "./Headers";

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
    // priority: RequestPriority;
    origin: string;
    referrer: string;
    // referrerPolicy: ReferrerPolicy;
    mode: RequestMode;
    useCORSPreflightFlag?: boolean;
    redirect: RequestRedirect;
    cache: RequestCache;
    // integrity: string;
    urlList: URL[];
    currentURL: URL;
    redirectCount: number;
    done?: boolean;
    timingAllowFailedFlag?: boolean;
};

// https://fetch.spec.whatwg.org/#request-class
// |----------------------------------------------------------|
// |                        Request                           |
// |----------------------------------------------------------|
const defaultInnerRequest = (parsedURL: URL): InnerRequest => {
    return {
        method: "GET",
        url: parsedURL,
        header_list: [],
        body: null,
        keepalive: false,
        // priority: "auto",
        origin: "client",
        referrer: "client",
        mode: "no-cors",
        redirect: "follow",
        cache: "default",
        urlList: [new URL(parsedURL.href)],
        get currentURL() {
            return this.urlList[this.urlList.length - 1];
        },
        redirectCount: 0,
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
            request = defaultInnerRequest(parsedURL);
            // 4.3. Set fallbackMode to "cors".
            fallbackMode = "cors";
        } else {
            assert(input instanceof Request, "Invalid Rquest input.");
            // 5.2. Set request to input’s request.
            request = input[_request];
            signal = input[_signal];
        }

        // TODO: ORIGIN
        // let origin = request.url.origin;
        request = {
            ...request,
            header_list: [...request.header_list],
            unsafeRequestFlag: false,
            urlList: [...request.urlList],
            // initiatorType: "fetch",
        };

        // 13. If init is not empty, then:
        if (init !== undefined) {
            // 13.1 If request’s mode is "navigate", then set it to "same-origin".
            if (request.mode === "navigate") {
                request.mode = "same-origin";
            }

            request.origin = "client";
            request.referrer = "client";
            // request.referrerPolicy = "";
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

        // request.referrerPolicy = init?.referrerPolicy ?? request.referrerPolicy;

        let mode = init?.mode ?? fallbackMode ?? "cors";
        if (request.mode === "navigate") throw new TypeError("Invalid mode.");
        request.mode = mode;

        // request.credentials = init?.credentials ?? request.credentials;
        request.cache = init?.cache ?? request.cache;
        if (
            request.cache === "only-if-cached" &&
            request.mode !== "same-origin"
        ) {
            throw new TypeError("Invalid cache mode.");
        }

        request.redirect = init?.redirect ?? request.redirect;
        // request.integrity = init?.integrity ?? request.integrity;
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

        // request.priority = init?.priority ?? request.priority;
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
            ((init?.body !== undefined && init.body !== null) ||
                inputBody !== null)
        ) {
            throw new TypeError("A GET/HEAD request cannot have a body.");
        }

        // 36. Let initBody be null.
        let initBody: ExtractedBody | null = null;
        // 37. If init["body"] exists and is non-null, then:
        if (init?.body !== undefined && init.body !== null) {
            // 37.1. Let bodyWithType be the result of extracting init["body"], with keepalive set to request’s keepalive.
            const bodyWithType = extractBody(
                init.body as any,
                request.keepalive,
            );
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
        if (
            inputOrInitBody !== null &&
            (inputOrInitBody as any).source === null
        ) {
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

    get mode(): RequestMode {
        return this[_request].mode;
    }

    get cache(): RequestCache {
        return this[_request].cache;
    }

    get redirect(): RequestRedirect {
        return this[_request].redirect;
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

    get credentials(): RequestCredentials {
        return "same-origin";
    }

    // TODO: Implement the clone method after supporting the tee method for ReadableStream
    // https://fetch.spec.whatwg.org/#ref-for-dom-request-clone
    clone(): Request {
        throw new Error("Request.clone method not implemented.");
    }
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

        return new URL(this.#url);
    }

    get header_list() {
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
            this.#redirectCount = op_http_request_redirect_count(
                this.requestResource,
            );
        }

        return this.#redirectCount;
    }

    get body(): Uint8Array | ExtractedBody | null {
        if (this.#body === undefined) {
            const requestBody = op_http_request_body(this.requestResource);
            if (requestBody instanceof Uint8Array) {
                this.#body = requestBody;
            } else if (requestBody == null) {
                this.#body = null;
            } else {
                const stream = decodedStreamToReadableStream(requestBody);
                const bodyWithType = extractBody(stream, this.keepalive);
                this.#body = bodyWithType.body;
            }
        }

        return this.#body;
    }

    get origin() {
        return "client";
    }

    get referrer() {
        return "client";
    }

    get mode(): RequestMode {
        return "cors";
    }

    get useCORSPreflightFlag() {
        return false;
    }

    get redirect(): RequestRedirect {
        return "follow";
    }

    get cache(): RequestCache {
        return "default";
    }

    get urlList() {
        return [this.url];
    }

    get currentURL() {
        return this.url;
    }
}

// create internal inner request
const createInnerRequestFromResource = (
    innerRequest: HttpRequestResource,
): InnerRequest => {
    const request = new InnerRequestResource(innerRequest);
    return request;
};

function toRequest(httpRequest: HttpRequestResource): Request {
    const innerRequest = createInnerRequestFromResource(httpRequest);
    const request = new Request(_illegalConstructor as any);
    request[_request] = innerRequest;
    mixinBody(request, _requestBody as any);
    return request;
}

function toInnerRequest(request: Request): InnerRequest {
    return request[_request];
}

export { InnerRequest, Request, RequestInfo, toInnerRequest, toRequest };
