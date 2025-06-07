import { DirEntry } from "@kedo/fs";
import {
    AbortController,
    AbortSignal,
    DOMException,
    Headers,
    Request,
    Response,
    TextDecoder,
    TextEncoder,
    URL,
    URLSearchParams,
    fetch,
    serve,
} from "@kedo/web";

globalThis.DOMException = DOMException;
globalThis.Headers = Headers;
globalThis.URL = URL;
globalThis.URLSearchParams = URLSearchParams;
globalThis.AbortController = AbortController;
globalThis.AbortSignal = AbortSignal;
globalThis.TextDecoder = TextDecoder;
globalThis.TextEncoder = TextEncoder;
globalThis.Request = Request;
globalThis.Response = Response;
globalThis.fetch = fetch;

Kedo.serve = serve;
Kedo.DirEntry = DirEntry;
