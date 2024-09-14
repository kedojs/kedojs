type HeadersGuard =
  | "none"
  | "immutable"
  | "request"
  | "request-no-cors"
  | "response";
declare module "@kedo/web/internals" {
  type HeadersInit = Headers | Record<string, string> | [string, string][];
  type ForEachCallback = (
    value: string,
    name: string,
    headers: Headers,
  ) => void;
  type PropertyNameKind = "KeyAndValue" | "Key" | "Value";

  const fillHeadersMapFrom: (
    headers: HeadersInit,
    headersMap: Headers,
    headersGuard?: HeadersGuard,
  ) => void;
  class Headers {
    [_headerList]: [string, string][];
    [_setCookieValues]: string[];
    [_headersGuard]: HeadersGuard;
    constructor(init: HeadersInit);
    append(name: string, value: string): void;
    delete(name: string): void;
    get(name: string): string;
    has(name: string): boolean;
    set(name: string, value: string): void;
    forEach(callback: ForEachCallback, thisArg?: any): void;
    [Symbol.toStringTag]: string;
    [_headersIterator](propertyNameKind?: PropertyNameKind): {
      [Symbol.iterator](): any;
      [Symbol.toStringTag]: string;
      next: () =>
        | {
            done: boolean;
            value?: undefined;
          }
        | {
            value: [string, string];
            done: boolean;
          }
        | {
            value: string;
            done: boolean;
          };
    };
    [Symbol.iterator](): {
      [Symbol.iterator](): any;
      [Symbol.toStringTag]: string;
      next: () =>
        | {
            done: boolean;
            value?: undefined;
          }
        | {
            value: [string, string];
            done: boolean;
          }
        | {
            value: string;
            done: boolean;
          };
    };
    getSetCookie(): string[];
    entries(): {
      [Symbol.iterator](): any;
      [Symbol.toStringTag]: string;
      next: () =>
        | {
            done: boolean;
            value?: undefined;
          }
        | {
            value: [string, string];
            done: boolean;
          }
        | {
            value: string;
            done: boolean;
          };
    };
    keys(): {
      [Symbol.iterator](): any;
      [Symbol.toStringTag]: string;
      next: () =>
        | {
            done: boolean;
            value?: undefined;
          }
        | {
            value: [string, string];
            done: boolean;
          }
        | {
            value: string;
            done: boolean;
          };
    };
    values(): {
      [Symbol.iterator](): any;
      [Symbol.toStringTag]: string;
      next: () =>
        | {
            done: boolean;
            value?: undefined;
          }
        | {
            value: [string, string];
            done: boolean;
          }
        | {
            value: string;
            done: boolean;
          };
    };
  }
  const emptyHeader: (headersMap: Headers) => void;

  const _abortReason: unique symbol;
  const _abortAlgorithms: unique symbol;
  const _dependent: unique symbol;
  const _sourceSignals: unique symbol;
  const _dependentSignals: unique symbol;
  const _signalAbort: unique symbol;
  const _addAlgorithm: unique symbol;
  const _removeAlgorithm: unique symbol;
  interface AbortAlgorithm {
    (): void;
  }
  class AbortSignal extends EventTarget {
    [_abortReason]: any;
    [_abortAlgorithms]: Set<AbortAlgorithm>;
    [_dependent]: boolean;
    [_sourceSignals]: IterableWeakSet<AbortSignal>;
    [_dependentSignals]: IterableWeakSet<AbortSignal>;
    constructor(key?: any);
    get aborted(): boolean;
    get reason(): any;
    throwIfAborted(): void;
    static abort(reason: any): AbortSignal;
    static timeout(ms: number): AbortSignal;
    static any(signals: AbortSignal[]): AbortSignal;
    set onabort(listener: EventListener);
    [_signalAbort](reason?: any): void;
    [_addAlgorithm](algorithm: AbortAlgorithm): void;
    [_removeAlgorithm](algorithm: AbortAlgorithm): void;
  }
  const createDependentAbortSignal: (signals: AbortSignal[]) => AbortSignal;
  const _signal: unique symbol;
  class AbortController {
    [_signal]: AbortSignal;
    constructor();
    get signal(): AbortSignal;
    abort(reason?: any): void;
  }

  const _urlObject: unique symbol;
  const _list: unique symbol;
  class URLSearchParams {
    [_list]: [string, string][];
    [_urlObject]: URL | null;
    constructor(init?: [string, string][] | Record<string, string> | string);
    private update;
    get size(): number;
    append(name: string, value: string): void;
    delete(name: string, value?: string): void;
    get(name: string): string | null;
    getAll(name: string): string[];
    has(name: string, value?: string): boolean;
    set(name: string, value: string): void;
    entries(): IterableIterator<[string, string]>;
    keys(): IterableIterator<string>;
    values(): IterableIterator<string>;
    sort(): void;
    toString(): string;
    [Symbol.iterator](): IterableIterator<[string, string]>;
  }
  const _urlRecord: unique symbol;
  const _queryObject: unique symbol;
  class URL {
    [_queryObject]: URLSearchParams;
    [_urlRecord]: UrlRecord;
    constructor(url: string, base?: string);
    static parse(url: string, base?: string): URL | null;
    static canParse(url: string, base?: string): boolean;
    get searchParams(): URLSearchParams;
    get origin(): string;
    get protocol(): string;
    set protocol(value: string);
    get username(): string;
    set username(value: string);
    get password(): string;
    set password(value: string);
    get host(): string;
    set host(value: string);
    get hostname(): string;
    set hostname(value: string);
    get port(): string;
    set port(value: string | null);
    get pathname(): string;
    set pathname(value: string);
    get search(): string;
    set search(value: string);
    get hash(): string;
    set hash(value: string);
    toJSON(): string;
    toString(): string;
    get href(): string;
    set href(value: string);
  }

  class DOMException extends Error {
    readonly name: string;
    readonly message: string;
    readonly code: number;
    static readonly INDEX_SIZE_ERR = 1;
    static readonly DOMSTRING_SIZE_ERR = 2;
    static readonly HIERARCHY_REQUEST_ERR = 3;
    static readonly WRONG_DOCUMENT_ERR = 4;
    static readonly INVALID_CHARACTER_ERR = 5;
    static readonly NO_DATA_ALLOWED_ERR = 6;
    static readonly NO_MODIFICATION_ALLOWED_ERR = 7;
    static readonly NOT_FOUND_ERR = 8;
    static readonly NOT_SUPPORTED_ERR = 9;
    static readonly INUSE_ATTRIBUTE_ERR = 10;
    static readonly INVALID_STATE_ERR = 11;
    static readonly SYNTAX_ERR = 12;
    static readonly INVALID_MODIFICATION_ERR = 13;
    static readonly NAMESPACE_ERR = 14;
    static readonly INVALID_ACCESS_ERR = 15;
    static readonly VALIDATION_ERR = 16;
    static readonly TYPE_MISMATCH_ERR = 17;
    static readonly SECURITY_ERR = 18;
    static readonly NETWORK_ERR = 19;
    static readonly ABORT_ERR = 20;
    static readonly URL_MISMATCH_ERR = 21;
    static readonly QUOTA_EXCEEDED_ERR = 22;
    static readonly TIMEOUT_ERR = 23;
    static readonly INVALID_NODE_TYPE_ERR = 24;
    static readonly DATA_CLONE_ERR = 25;
    private static readonly errorCodes;
    constructor(message?: string, name?: string);
    toJSON(): {
      name: string;
      message: string;
      code: number;
    };
    static fromJSON(serialized: {
      name: string;
      message: string;
    }): DOMException;
  }

  export {
    Headers,
    fillHeadersMapFrom,
    emptyHeader,
    AbortSignal,
    AbortController,
    URLSearchParams,
    URL,
    DOMException,
    createDependentAbortSignal,
  };
}
