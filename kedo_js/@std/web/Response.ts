import { readableStreamResource } from "@kedo:int/std/stream";
import { extractBody, mixinBody } from "./Body";
import {
    _headers,
    _illegalConstructor,
    _response,
    _responseBody,
    isExtractedBody
} from "./commons";
import { fillHeadersMapFrom, headerInnerList, Headers } from "./Headers";

// https://fetch.spec.whatwg.org/#response-class
// |----------------------------------------------------------|
// |                        Response                          |
// |----------------------------------------------------------|
type InnerResponse = {
    type: ResponseType;
    aborted?: boolean;
    url: URL | null;
    urlList: URL[];
    status: number;
    statusMessage: string;
    headerList: [string, string][];
    body: Uint8Array | ExtractedBody | null;
    // cacheState: "" | "local" | "validated";
};

const defaultInnerResponse = (): InnerResponse => {
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
        // cacheState: "",
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
        fillHeadersMapFrom(init.headers as Headers, response[_headers], "response");
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
        this[_response] = defaultInnerResponse();
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
        response[_response] = defaultInnerResponse();
        response[_headers] = new Headers(response[_response].headerList);
        initializeResponse(response, init, bodyWithType);
        mixinBody(response, _responseBody as any);
        return response;
    }

    static error(): Response {
        const response = new Response(_illegalConstructor as any);
        response[_response] = defaultInnerResponse();
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
        response[_response] = defaultInnerResponse();
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

function toInnerResponse(response: Response): InnerResponse {
    return response[_response];
}

function toHttpResponse(response: Response): HttpResponse {
    let headersList: [string, string][] = [];
    if (response.headers instanceof Headers) {
        headersList = headerInnerList(response.headers);
    }

    let innerResponse = toInnerResponse(response);
    const httpResponse: HttpResponse = {
        url: response.url || "/",
        status: innerResponse.status,
        headers: headersList,
    };

    if (innerResponse.body && isExtractedBody(innerResponse.body)) {
        if (innerResponse.body.source) {
            httpResponse.source = innerResponse.body.source;
        } else if (innerResponse.body.stream) {
            httpResponse.stream = readableStreamResource(innerResponse.body.stream);
        }
    }

    return httpResponse;
}

export {
    createResponse,
    InnerResponse,
    Response,
    toHttpResponse,
    toInnerResponse
};
