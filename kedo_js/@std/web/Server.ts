import { asyncOp } from "@kedo/utils";
import {
    InternalSignal,
    op_internal_start_server,
    op_read_async_request_event,
    op_read_request_event,
    op_send_event_response,
    op_send_signal,
} from "@kedo:op/web";
import { AbortSignal } from "./AbortSignal";
import { Request, toRequest } from "./Request";
import { Response, toHttpResponse } from "./Response";
import { StreamError } from "@kedo:int/std/stream";

// ------------------------------------------------------------
// |                        Http Server                       |
// ------------------------------------------------------------
type ServeOptions = {
    hostname?: string;
    port?: number;
    signal?: AbortSignal;
    onListen?: (args: any) => void;
    onError?: OnErrorHandler;
    handler?: ServerHandler;
};

type OnErrorHandler = (error: any) => Response | Promise<Response>;
type ServerHandler = (request: Request) => Response | Promise<Response>;

// Predefined responses to avoid creating new objects
const INTERNAL_SERVER_ERROR: HttpResponse = Object.freeze({
    status: 500,
    url: "/",
    headers: [["content-type", "text/plain"]],
} as HttpResponse);

function formatAddress(address: string): [string, string] {
    // Handle IPv6 addresses in brackets
    const ipv6Match = address.match(/^\[(.+)\]:(\d+)$/);
    if (ipv6Match) {
        return [`[${ipv6Match[1]}]`, ipv6Match[2]];
    }

    // Handle IPv4 addresses and regular hostnames
    const lastColonIndex = address.lastIndexOf(":");
    if (lastColonIndex === -1) {
        throw new Error("Invalid address format: missing port");
    }

    const hostname = address.substring(0, lastColonIndex);
    const port = address.substring(lastColonIndex + 1);

    return [hostname, port];
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

    const onError = serverOptions?.onError;
    // prepare the internal options
    let internalOptions: InternalServerOptions = {
        onError,
        signal: internalSignal,
        key: tlsCertificate?.key,
        cert: tlsCertificate?.cert,
        port: serverOptions?.port || 8080,
        hostname: serverOptions?.hostname || "0.0.0.0",
    };

    asyncOp(op_internal_start_server, internalOptions)
        .then(({ reader, address }) => {
            const [hostname, port] = formatAddress(address);
            serverOptions?.onListen?.({
                hostname: hostname,
                port: port,
                key: internalOptions.key,
                cert: internalOptions.cert,
            });

            return processRequests(reader, handler, onError);
        })
        .catch((error) => {
            op_send_signal(internalSignal);
            throw error;
        });
}

function processRequests(
    channel: NetworkBufferChannelReaderResource,
    handler: ServerHandler,
    onError?: OnErrorHandler,
): Promise<void> {
    const internalHandler = serverHandler(handler, onError);

    return (async () => {
        while (true) {
            let event = op_read_request_event(channel);
            if (event === StreamError.Empty) {
                event = await asyncOp(op_read_async_request_event, channel);
            }

            if (event === null || typeof event !== "object") {
                break;
            }

            internalHandler(event.request, event.sender);
        }
    })();
}

function serverHandler(
    handler: ServerHandler,
    onError?: OnErrorHandler,
): InternalServerHandler {
    const asyncHandler = async (request: HttpRequestResource) => {
        const requestObject = toRequest(request);
        const response = await handler(requestObject);
        return response;
    };

    const asyncErrorHandler = async (error: any): Promise<HttpResponse> => {
        try {
            const response = await onError!(error);
            const httpResponse = toHttpResponse(response);
            return httpResponse;
        } catch (error) {
            return INTERNAL_SERVER_ERROR;
        }
    };

    const internalHandler = (
        request: HttpRequestResource,
        sender: RequestEventResource,
    ) => {
        asyncHandler(request)
            .then((response) => {
                const httpResponse = toHttpResponse(response);
                op_send_event_response(sender, httpResponse);
            })
            .catch((error) => {
                if (onError === undefined) {
                    console.log("Error: ", error.message);
                    op_send_event_response(sender, INTERNAL_SERVER_ERROR);
                    return;
                }

                asyncErrorHandler(error).then((res) =>
                    op_send_event_response(sender, res),
                );
            });
    };

    return internalHandler;
}

export { serve };
