import { asyncOp } from "@kedo/utils";
import { ReadableStream } from "@kedo:int/std/stream";
import { op_read_decoded_stream } from "@kedo:op/web";

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

// TODO: Move this to a the int stream module
// we need to also optimize this function
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

export {
    _headers,
    _illegalConstructor,
    _internalHeaders,
    _request,
    _requestBody,
    _response,
    _responseBody,
    _signal,
    decodedStreamToReadableStream,
    HTTP_METHODS,
    isExtractedBody
};
