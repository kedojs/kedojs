type DecodedBodyStream = {};

type RequestEventResource = {};

type HttpResponse = {
    readonly body?: DecodedBodyStream;
    readonly headers: [string, string][];
    readonly status: number;
    readonly status_message: string;
    readonly url: string;
};

type HttpRequest = {
    stream?: import("@kedo:op/web").ReadableStreamResource;
    source?: Uint8Array | null;
    signal?: import("@kedo:op/web").InternalSignal;
    redirect?: number;
    readonly header_list: [string, string][];
    readonly method: string;
    readonly url: string;
};

declare class HttpRequestResource {
    constructor();
}

type InternalServerHandler = (
    request: HttpRequestResource,
    sender: RequestEventResource,
) => void;

type InternalServerOptions = {
    hostname: string;
    port: number;
    key?: string;
    cert?: string;
    signal?: import("@kedo:op/web").InternalSignal;
    handler: InternalServerHandler;
};

declare module "@kedo:op/web" {

    class FetchClient {
        constructor();
    }

    export function is_array_buffer_detached(buffer: ArrayBufferLike): boolean;
    export function parse_url_encoded_form(body: string): [string, string][];
    export function serialize_url_encoded_form(data: [string, string][]): string;
    export function encoding_for_label_no_replacement(label: string): string;
    export function op_send_signal(signal: InternalSignal): void;
    export function op_read_sync_readable_stream(
        reader: ReadableStreamResourceReader,
    ): Uint8Array | undefined;
    export function op_write_sync_readable_stream(
        resource: ReadableStreamResource,
        chunk: Uint8Array,
    ): void;
    export function op_read_readable_stream(
        reader: ReadableStreamResourceReader,
        callback: OpStyleCallback<Uint8Array | undefined>,
    ): void;
    export function op_read_decoded_stream(
        resource: DecodedBodyStream,
        callback: OpStyleCallback<Uint8Array | undefined>,
    ): void;
    export function op_new_fetch_client(): FetchClient;
    export function op_internal_fetch(
        client: FetchClient,
        request: HttpRequest,
        callback: OpStyleCallback<HttpResponse>,
    ): void;
    export function op_internal_start_server(
        options: InternalServerOptions,
        callback: OpStyleCallback<string>,
    ): void;
    export function op_send_event_response(
        sender: RequestEventResource,
        response: HttpResponse,
    ): void;
    export function op_write_readable_stream(
        resource: ReadableStreamResource,
        chunk: Uint8Array,
        callback: OpStyleCallback<number>,
    ): void;
    export function op_close_stream_resource(
        resource: ReadableStreamResource,
    ): void;
    export function op_acquire_stream_reader(
        resource: ReadableStreamResource,
    ): ReadableStreamResourceReader;
    export function op_wait_close_readable_stream(
        resource: ReadableStreamResource,
        blocking: boolean,
        callback: OpStyleCallback<void>,
    ): void;
    export class ReadableStreamResourceReader { }
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
    export class InternalSignal {
        constructor();
    }

    export class ReadableStreamResource {
        constructor(hwm: number);
    }

    // http request resource
    export function op_http_request_method(_: HttpRequestResource): string;
    export function op_http_request_uri(_: HttpRequestResource): string;
    export function op_http_request_headers(_: HttpRequestResource): [string, string][];
    export function op_http_request_keep_alive(_: HttpRequestResource): boolean;
    export function op_http_request_redirect(_: HttpRequestResource): number;
    export function op_http_request_redirect_count(_: HttpRequestResource): number;
    export function op_http_request_body(_: HttpRequestResource): { stream?: ReadableStreamResource; source?: Uint8Array } | null;
}

declare module "@kedo:op/fs" {
    import { DirEntry } from "@kedo/fs";

    export function op_fs_read_file_sync(path: string): string;
    export function op_fs_read_dir_sync(path: string): DirEntry[];
    export function op_fs_write_file_sync(path: string, data: string): void;
    export function op_fs_remove_sync(path: string, recursive: boolean): void;
    export function op_fs_read_file(
        path: string,
        callback: OpStyleCallback<string>,
    ): void;
    export function op_fs_write_file(
        path: string,
        data: string,
        callback: OpStyleCallback<void>,
    ): void;
    export function op_fs_read_dir(
        path: string,
        callback: OpStyleCallback<DirEntry>,
    ): void;
    export function op_fs_remove(
        path: string,
        recursive: boolean,
        callback: OpStyleCallback<void>,
    ): void;
}
