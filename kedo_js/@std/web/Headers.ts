const _headerList = Symbol("headerList");
const _setCookieValues = Symbol("setCookieValues");
const _headersGuard = Symbol("headersGuard");
const _headersIterator = Symbol("headersIterator");

type HeadersInit = Headers | Record<string, string> | [string, string][];

type ForEachCallback = (value: string, name: string, headers: Headers) => void;

const isHTTPWhitespace = (characterCode: number) => {
  // Checks for HTTP tab (0x09), space (0x20), LF (0x0A), and CR (0x0D)
  return (
    characterCode === 0x09 ||
    characterCode === 0x20 ||
    characterCode === 0x0a ||
    characterCode === 0x0d
  );
};

// See https://fetch.spec.whatwg.org/#concept-header
const isValidHTTPHeaderValue = (value: string) => {
  if (value.length === 0) return false;

  // Check for leading and trailing HTTP whitespace bytes
  if (
    isHTTPWhitespace(value.charCodeAt(0)) ||
    isHTTPWhitespace(value.charCodeAt(value.length - 1))
  ) {
    return false;
  }

  // Check for forbidden characters (NUL, LF, CR) inside the value
  for (let i = 0; i < value.length; i++) {
    let charCode = value.charCodeAt(i);
    if (charCode === 0x00 || charCode === 0x0a || charCode === 0x0d) {
      return false;
    }
  }

  return true;
};

const HTTP_HEADER_TOKEN_REGEXP = /^[a-zA-Z0-9!#$%&'*+\-.^_`|~]+$/;

// See RFC 7230, Section 3.2.6.
const isValidHTTPToken = (value: string) => {
  if (!value) {
    return false;
  }

  // Regular expression to match valid token characters as per RFC 7230, Section 3.2.6
  return HTTP_HEADER_TOKEN_REGEXP.test(value);
};

// https://fetch.spec.whatwg.org/#concept-headers-fill
const fillHeadersMap = (headersInit: HeadersInit, headersMap: Headers) => {
  if (Array.isArray(headersInit)) {
    for (let i = 0; i < headersInit.length; i++) {
      if (headersInit[i].length !== 2) {
        throw new TypeError(
          "Header sub-sequence must contain exactly two items",
        );
      }

      appendToHeaderMap(headersMap, headersInit[i][0], headersInit[i][1]);
    }
  } else if (headersInit instanceof Headers) {
    headersInit.forEach((value, name) => {
      appendToHeaderMap(headersMap, name, value);
    });
  } else {
    for (let key in headersInit) {
      appendToHeaderMap(headersMap, key, headersInit[key]);
    }
  }
};

const fillHeadersMapFrom = (
  headers: HeadersInit,
  headersMap: Headers,
  headersGuard: HeadersGuard = "none",
) => {
  fillHeadersMap(headers, headersMap);
  headersMap[_headersGuard] = headersGuard;
};

const appendSetCookie = (
  value: string,
  setCookieValues: string[],
  headersGuard: HeadersGuard,
) => {
  if (!isValidHTTPHeaderValue(value))
    throw new TypeError("Header 'Set-Cookie' has invalid value: " + value);

  if (headersGuard === "immutable")
    throw new TypeError("Headers object is immutable");

  setCookieValues.push(value);
};

const findHeaderIndex = (name: string, headersMap: Headers) => {
  for (let i = 0; i < headersMap[_headerList].length; i++) {
    if (headersMap[_headerList][i][0] === name) return i;
  }

  return -1;
};

const lowerCase = (name: string) => {
  return name.toLowerCase();
};

const appendToHeaderMap = (
  headersMap: Headers,
  name: string,
  value: string,
) => {
  const normalizedValue = value.trim();
  if (name === "set-cookie")
    return appendSetCookie(
      normalizedValue,
      headersMap[_setCookieValues],
      headersMap[_headersGuard],
    );

  if (!isValidHTTPHeaderValue(normalizedValue))
    throw new TypeError("Header value is invalid");

  let headerIndex = findHeaderIndex(name, headersMap);
  if (headerIndex !== -1) {
    headersMap[_headerList][headerIndex][1] += ", " + normalizedValue;
    return;
  }

  if (!isValidHTTPToken(name)) throw new TypeError("Header name is invalid");

  if (headersMap[_headersGuard] === "immutable")
    throw new TypeError("Headers object is immutable");

  Array.prototype.push.call(headersMap[_headerList], [
    lowerCase(name),
    normalizedValue,
  ]);
};

/**
 * Represents a collection of HTTP headers with methods to manage
 * header name-value pairs, including support for adding, retrieving,
 * and removing values.
 *
 * @remarks
 * The class also facilitates iteration over the headers and provides
 * support for retrieving set-cookie values.
 *
 * @example
 * ```ts
 * const headers = new Headers({ 'Content-Type': 'application/json' });
 * headers.append('Authorization', 'Bearer token');
 * console.log(headers.get('Authorization')); // 'Bearer token'
 * ```
 */
class Headers {
  [_headerList]: [string, string][] = [];
  [_setCookieValues]: string[] = [];
  [_headersGuard]: HeadersGuard = "none"; // none, immutable, request, request-no-cors, response

  constructor(init: HeadersInit) {
    if (init) {
      fillHeadersMap(init, this);
    }
  }

  append(name: string, value: string) {
    appendToHeaderMap(this, lowerCase(name), value);
  }

  delete(name: string) {
    if (this[_headersGuard] === "immutable")
      throw new TypeError("Headers object is immutable");

    name = lowerCase(name);

    if (!isValidHTTPToken(name)) throw new TypeError("Header name is invalid");

    if (name === "set-cookie") {
      this[_setCookieValues] = [];
      return;
    }

    let headerIndex = findHeaderIndex(name, this);
    if (headerIndex !== -1) {
      this[_headerList].splice(headerIndex, 1);
    }
  }

  get(name: string) {
    name = lowerCase(name);
    if (!isValidHTTPToken(name)) return null;

    if (name === "set-cookie") {
      return this[_setCookieValues].join(", ");
    }

    let headerIndex = findHeaderIndex(name, this);
    if (headerIndex === -1) return null;

    return this[_headerList][headerIndex][1];
  }

  has(name: string) {
    name = lowerCase(name);
    if (!isValidHTTPToken(name)) return false;

    if (name === "set-cookie") return this[_setCookieValues].length > 0;

    return findHeaderIndex(name, this) !== -1;
  }

  set(name: string, value: string) {
    name = lowerCase(name);
    if (!isValidHTTPToken(name)) throw new TypeError("Header name is invalid");

    if (!isValidHTTPHeaderValue(value))
      throw new TypeError("Header value is invalid");

    if (this[_headersGuard] === "immutable")
      throw new TypeError("Headers object is immutable");

    if (name === "set-cookie") {
      this[_setCookieValues] = [];
      return appendSetCookie(
        value,
        this[_setCookieValues],
        this[_headersGuard],
      );
    }

    let headerIndex = findHeaderIndex(name, this);
    if (headerIndex !== -1) {
      this[_headerList][headerIndex][1] = value;
    }
  }

  forEach(callback: ForEachCallback, thisArg?: any) {
    for (let i = 0; i < this[_headerList].length; i++) {
      callback.call(
        thisArg,
        this[_headerList][i][1],
        this[_headerList][i][0],
        this,
      );
    }

    if (this[_setCookieValues].length > 0) {
      callback.call(
        thisArg,
        this[_setCookieValues].join(", "),
        "set-cookie",
        this,
      );
    }
  }

  [Symbol.toStringTag] = "Headers";

  [_headersIterator](propertyNameKind: PropertyNameKind = "KeyAndValue") {
    let index = 0;
    let cookiesVisited = false;
    return {
      [Symbol.iterator]() {
        return this;
      },
      [Symbol.toStringTag]: "HeadersIterator",
      next: (): IteratorResult<string | [string, string] | undefined> => {
        let header: [string, string];
        if (index >= this[_headerList].length) {
          if (this[_setCookieValues].length > 0 && !cookiesVisited) {
            header = ["set-cookie", this[_setCookieValues].join(", ")];
            cookiesVisited = true;
          } else return { value: undefined, done: true };
        } else header = this[_headerList][index++];

        if (propertyNameKind === "KeyAndValue") {
          return { value: header, done: false };
        } else if (propertyNameKind === "Key") {
          return { value: header[0], done: false };
        } else if (propertyNameKind === "Value") {
          return { value: header[1], done: false };
        }

        throw new TypeError("Invalid PropertyNameKind");
      },
    };
  }

  [Symbol.iterator]() {
    return this[_headersIterator]();
  }

  getSetCookie() {
    return [...this[_setCookieValues]];
  }

  entries(): IterableIterator<[string, string]> {
    return this[_headersIterator]() as IterableIterator<[string, string]>;
  }

  keys(): IterableIterator<string> {
    return this[_headersIterator]("Key") as IterableIterator<string>;
  }

  values(): IterableIterator<string> {
    return this[_headersIterator]("Value") as IterableIterator<string>;
  }
}

const emptyHeader = (headersMap: Headers) => {
  headersMap[_headerList] = [];
  headersMap[_setCookieValues] = [];
};

const headerInnerList = (headersMap: Headers) => {
  return headersMap[_headerList];
}

const headersFromInnerList = (innerList: [string, string][]) => {
  const headers = new Headers(null as any);
  headers[_headerList] = innerList;
  headers[_headersGuard] = "none";
  return headers;
}

export { emptyHeader, fillHeadersMapFrom, headerInnerList, Headers, headersFromInnerList };
