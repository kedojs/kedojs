declare module "@kedo/web/internals" {
  type HeadersInit = Headers | Record<string, string> | [string, string][];
  type ForEachCallback = (
    value: string,
    name: string,
    headers: Headers,
  ) => void;
  type PropertyNameKind = "KeyAndValue" | "Key" | "Value";
  type HeadersGuard =
    | "none"
    | "immutable"
    | "request"
    | "request-no-cors"
    | "response";
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
  export { Headers, fillHeadersMapFrom, emptyHeader };
}
