import { asyncOp } from "@kedo/utils";
import {
    InternalSignal,
    op_internal_start_server,
    op_send_event_response,
    op_send_signal
} from "@kedo:op/web";
import { AbortSignal } from "./AbortSignal";
import { Request, toRequest } from "./Request";
import { Response, toHttpResponse } from "./Response";

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

function internalServerError(): HttpResponse {
    return {
        status: 500,
        url: "/",
        status_message: "Internal Server Error",
        headers: [
            ["content-type", "text/plain"],
        ],
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

    const onError = serverOptions?.onError;
    // prepare the internal options
    let internalOptions: InternalServerOptions = {
        onError,
        signal: internalSignal,
        key: tlsCertificate?.key,
        cert: tlsCertificate?.cert,
        port: serverOptions?.port || 8080,
        handler: serverHandler(handler, onError),
        hostname: serverOptions?.hostname || "0.0.0.0",
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

function serverHandler(handler: ServerHandler, onError?: OnErrorHandler): InternalServerHandler {
    const asyncHandler = async (request: Request) => {
        const response = await handler(request);
        return response;
    };

    const asyncErrorHandler = async (error: any, sender: RequestEventResource): Promise<HttpResponse> => {
        try {
            const response = await onError!(error);
            const httpResponse = toHttpResponse(response);
            return httpResponse;
        } catch (error) {
            return internalServerError();
        }
    }

    const internalHandler = (request: HttpRequestResource, sender: RequestEventResource) => {
        const requestObject = toRequest(request);
        asyncHandler(requestObject)
            .then((response) => {
                const httpResponse = toHttpResponse(response);
                op_send_event_response(sender, httpResponse);
            })
            .catch((error) => {
                if (onError === undefined) {
                    console.log("Error: ", error.message);
                    op_send_event_response(sender, internalServerError());
                    return;
                }

                asyncErrorHandler(error, sender).then((res) => op_send_event_response(sender, res));
            });
    }

    return internalHandler;
}

export { serve };
