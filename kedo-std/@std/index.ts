// import { Headers } from "./web/Headers.js";
// import { URLSearchParams, URL } from "./web/URL.js";
// import { DOMException } from "./web/DOMException.js";
import { Request, Response, fetch } from "./web/Fetch";
import { TextDecoder, TextEncoder } from "./web/TextDecoder";
// import { AbortSignal, AbortController } from "./web/AbortSignal.js";
import {
  AbortSignal,
  AbortController,
  Headers,
  URL,
  URLSearchParams,
  DOMException,
} from "@kedo/web/internals";

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