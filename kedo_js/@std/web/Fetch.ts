
import { asyncOp } from "@kedo/utils";
import {
    isInReadableState,
    readableStreamResource
} from "@kedo:int/std/stream";
import {
    InternalSignal,
    op_internal_fetch,
    op_new_fetch_client,
    op_send_signal
} from "@kedo:op/web";
import {
    decodedStreamToReadableStream,
    isExtractedBody
} from "./commons";
import { InnerRequest, Request, RequestInfo, toInnerRequest } from "./Request";
import { createResponse, InnerResponse, Response } from "./Response";

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
        const request = toInnerRequest(requestObject);

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
    if (request.body && isExtractedBody(request.body)) {
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

export { fetch };
