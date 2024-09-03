import {
  parse_url_encoded_form,
  serialize_url_encoded_form,
  UrlRecord,
} from "@kedo/internal/utils";

// | -------------------------------------------- |
// | https://url.spec.whatwg.org/#urlsearchparams |
// |              URLSearchParams                 |
// | -------------------------------------------- |
const _urlObject = Symbol("[urlObject]");
const _list = Symbol("[list]");

class URLSearchParams {
  [_list]: [string, string][];
  [_urlObject]: URL | null;

  constructor(init: [string, string][] | Record<string, string> | string = "") {
    this[_list] = [];
    this[_urlObject] = null;

    if (typeof init === "string") {
      if (init.startsWith("?")) {
        init = init.substring(1);
      }

      this[_list] = parse_url_encoded_form(init);
    } else if (Array.isArray(init)) {
      // 1. If init is a sequence, then for each innerSequence of init:
      for (const innerSequence of init) {
        // 1.1. If innerSequence’s size is not 2, then throw a TypeError.
        if (innerSequence.length !== 2) {
          throw new TypeError(
            "Each inner sequence must have exactly 2 elements",
          );
        }
        // 1.2. Append (innerSequence[0], innerSequence[1]) to query’s list.
        this.append(innerSequence[0], innerSequence[1]);
      }
    } else {
      // Otherwise, if init is a record, then for each name → value of init, append (name, value) to query’s list.
      for (const [name, value] of Object.entries(init)) {
        this.append(name, value);
      }
    }
  }

  private update() {
    if (this[_urlObject] === null) {
      return;
    }

    let serializedQuery: string | null = this.toString();
    if (serializedQuery === "") {
      serializedQuery = null;
      return;
    }
    this[_urlObject][_urlRecord].set("query", serializedQuery);
    if (serializedQuery === null) {
      potentiallyStripTrailingSpacesFromOpaquePath(this[_urlObject]);
    }
  }

  get size(): number {
    return this[_list].length;
  }

  append(name: string, value: string): void {
    this[_list].push([name, value]);
    this.update();
  }

  delete(name: string, value?: string): void {
    if (value !== undefined) {
      this[_list] = this[_list].filter(([n, v]) => n !== name || v !== value);
    } else {
      this[_list] = this[_list].filter(([n]) => n !== name);
    }
    this.update();
  }

  get(name: string): string | null {
    const found = this[_list].find(([n]) => n === name);
    return found ? found[1] : null;
  }

  getAll(name: string): string[] {
    return this[_list].filter(([n]) => n === name).map(([, v]) => v);
  }

  has(name: string, value?: string): boolean {
    if (value !== undefined) {
      return this[_list].some(([n, v]) => n === name && v === value);
    } else {
      return this[_list].some(([n]) => n === name);
    }
  }

  set(name: string, value: string): void {
    const index = this[_list].findIndex(([n]) => n === name);
    if (index !== -1) {
      this[_list][index] = [name, value];
      this[_list] = this[_list].filter(([n], i) => n !== name || i === index);
    } else {
      this[_list].push([name, value]);
    }
    this.update();
  }

  entries(): IterableIterator<[string, string]> {
    return this[Symbol.iterator]();
  }

  keys(): IterableIterator<string> {
    return this[_list].map(([name]) => name)[Symbol.iterator]();
  }

  values(): IterableIterator<string> {
    return this[_list].map(([, value]) => value)[Symbol.iterator]();
  }

  sort(): void {
    this[_list].sort(([a], [b]) => a.localeCompare(b));
    this.update();
  }

  toString(): string {
    return serialize_url_encoded_form(this[_list]);
  }

  *[Symbol.iterator](): IterableIterator<[string, string]> {
    for (const pair of this[_list]) {
      yield pair;
    }
  }
}

// | -------------------------------------- |
// | https://url.spec.whatwg.org/#url-class |
// |                  URL                   |
// | -------------------------------------- |

const _urlRecord = Symbol("[urlRecord]");
const _queryObject = Symbol("[queryObject]");

const cannotHaveAUsernamePasswordPort = (urlRecord: UrlRecord): boolean => {
  const isSchemeFileOrData =
    urlRecord.get("scheme") === "file" || urlRecord.get("scheme") === "data";
  const isHostNull =
    urlRecord.get("host") === null || urlRecord.get("host") === "";
  return isSchemeFileOrData || isHostNull;
};

// const initializeUrl = (url: URL, urlRecord: UrlRecord) => {
//   // 1. Let query be urlRecord’s query, if that is non-null; otherwise the empty string.
//   const query = urlRecord.get("query") || "";
//   // 2. Set url’s URL to urlRecord.
//   url[_urlRecord] = urlRecord;
//   // 3. Set url’s query object to a new URLSearchParams object.
//   // 4. Initialize url’s query object with query.
//   url[_queryObject] = new URLSearchParams(query);
//   // 5. Set url’s query object’s URL object to url.
//   url[_queryObject][_urlObject] = url;
// };

// A URL path segment is an ASCII string. It commonly refers to a directory or a file, but has no predefined meaning.
// A single-dot URL path segment is a URL path segment that is "." or an ASCII case-insensitive match for "%2e".
// A double-dot URL path segment is a URL path segment that is ".." or an ASCII case-insensitive match for ".%2e", "%2e.", or "%2e%2e".
// A URL has an opaque path if its path is a URL path segment.
const isOpaquePath = (urlRecord: UrlRecord): boolean => {
  const path = urlRecord.get("path");
  return path === "." || path === "..";
};

// To potentially strip trailing spaces from an opaque path given a URL object url:
// If url’s URL does not have an opaque path, then return.
// If url’s URL’s fragment is non-null, then return.
// If url’s URL’s query is non-null, then return.
// Remove all trailing U+0020 SPACE code points from url’s URL’s path.
const potentiallyStripTrailingSpacesFromOpaquePath = (url: URL) => {
  if (!isOpaquePath(url[_urlRecord])) return;
  if (url[_urlRecord].get("fragment") !== null) return;
  if (url[_urlRecord].get("query") !== null) return;

  const path = url[_urlRecord].get("path") || "";
  url[_urlRecord].set("path", path.replace(/ +$/, ""));
};

class URL {
  [_queryObject]: URLSearchParams;
  [_urlRecord]: UrlRecord;

  constructor(url: string, base?: string) {
    const parsedURL = new UrlRecord(url, base);
    // initializeUrl(this, parsedURL);
    const query = parsedURL.get("query") || "";
    this[_urlRecord] = parsedURL;
    this[_queryObject] = new URLSearchParams(query);
    this[_queryObject][_urlObject] = this;
  }

  static parse(url: string, base?: string): URL | null {
    try {
      return new URL(url, base);
    } catch {
      // 2. If parsedURL is failure, then return null.
      return null;
    }
  }

  static canParse(url: string, base?: string): boolean {
    try {
      new URL(url, base);
      return true;
    } catch {
      return false;
    }
  }

  get searchParams(): URLSearchParams {
    return this[_queryObject];
  }

  get origin(): string {
    return this[_urlRecord].get("origin")!;
  }

  get protocol(): string {
    return `${this[_urlRecord].get("scheme")!}:`;
  }

  set protocol(value: string) {
    this[_urlRecord].set("scheme", value);
  }

  // The username getter steps are to return this’s URL’s username.
  get username(): string {
    return this[_urlRecord].get("username")!;
  }

  // The username setter steps are:
  // 1. If this’s URL cannot have a username/password/port, then return.
  // 2. Set the username given this’s URL and the given value.
  set username(value: string) {
    if (cannotHaveAUsernamePasswordPort(this[_urlRecord])) {
      return;
    }

    this[_urlRecord].set("username", value);
  }

  // 1. The password getter steps are to return this’s URL’s password.
  get password(): string {
    return this[_urlRecord].get("password")!;
  }

  // The password setter steps are:
  // If this’s URL cannot have a username/password/port, then return.
  // Set the password given this’s URL and the given value.
  set password(value: string) {
    if (cannotHaveAUsernamePasswordPort(this[_urlRecord])) {
      return;
    }

    this[_urlRecord].set("password", value);
  }

  // The host getter steps are:
  // Let url be this’s URL.
  // If url’s host is null, then return the empty string.
  // If url’s port is null, return url’s host, serialized.
  // Return url’s host, serialized, followed by U+003A (:) and url’s port, serialized.
  get host(): string {
    const url = this[_urlRecord];
    const host = url.get("host");
    if (host === null) {
      return "";
    }

    const port = url.get("port");
    if (port === null) {
      return host;
    }

    return `${host}:${port}`;
  }

  // The host setter steps are:
  // If this’s URL has an opaque path, then return.
  // Basic URL parse the given value with this’s URL as url and host state as state override.
  set host(value: string) {
    if (isOpaquePath(this[_urlRecord])) {
      return;
    }

    const [host, port] = value.split(":");
    this[_urlRecord].set("host", host);
    if (port) {
      this[_urlRecord].set("port", port);
    }
  }

  // The hostname getter steps are:
  // If this’s URL’s host is null, then return the empty string.
  // Return this’s URL’s host, serialized.
  get hostname(): string {
    return this[_urlRecord].get("host") || "";
  }

  // The hostname setter steps are:
  // If this’s URL has an opaque path, then return.
  // Basic URL parse the given value with this’s URL as url and hostname state as state override.
  set hostname(value: string) {
    if (isOpaquePath(this[_urlRecord])) {
      return;
    }

    this[_urlRecord].set("host", value);
  }

  get port(): string {
    return this[_urlRecord].get("port") || "";
  }

  set port(value: string | null) {
    if (cannotHaveAUsernamePasswordPort(this[_urlRecord])) {
      return;
    }

    if (value === "" || value === null) {
      this[_urlRecord].set("port", null as any);
      return;
    }

    this[_urlRecord].set("port", value);
  }

  get pathname(): string {
    return this[_urlRecord].get("path") || "";
  }

  set pathname(value: string) {
    if (isOpaquePath(this[_urlRecord])) {
      return;
    }

    this[_urlRecord].set("path", value);
  }

  // The search getter steps are:
  // If this’s URL’s query is either null or the empty string, then return the empty string.
  // Return U+003F (?), followed by this’s URL’s query.
  get search(): string {
    const query = this[_urlRecord].get("query");
    return query ? `?${query}` : "";
  }

  // The search setter steps are:
  // Let url be this’s URL.
  // If the given value is the empty string:
  //  Set url’s query to null.
  //  Empty this’s query object’s list.
  //  Potentially strip trailing spaces from an opaque path with this.
  //  Return.
  // Let input be the given value with a single leading U+003F (?) removed, if any.
  // Set url’s query to the empty string.
  // Basic URL parse input with url as url and query state as state override.
  // Set this’s query object’s list to the result of parsing input.
  set search(value: string) {
    const url = this[_urlRecord];
    if (value === "") {
      url.set("query", null as any);
      this[_queryObject][_list] = [];
      potentiallyStripTrailingSpacesFromOpaquePath(this);
      return;
    }

    const input = value[0] === "?" ? value.slice(1) : value;
    url.set("query", input);
    this[_queryObject][_list] = parse_url_encoded_form(input);
  }

  get hash(): string {
    const fragment = this[_urlRecord].get("fragment");
    return fragment ? `#${fragment}` : "";
  }

  // The hash setter steps are:
  // If the given value is the empty string:
  //  Set this’s URL’s fragment to null.
  //  Potentially strip trailing spaces from an opaque path with this.
  //  Return.
  // Let input be the given value with a single leading U+0023 (#) removed, if any.
  // Set this’s URL’s fragment to the empty string.
  // Basic URL parse input with this’s URL as url and fragment state as state override.
  set hash(value: string) {
    const url = this[_urlRecord];
    if (value === "") {
      url.set("fragment", null as any);
      potentiallyStripTrailingSpacesFromOpaquePath(this);
      return;
    }

    const input = value[0] === "#" ? value.slice(1) : value;
    url.set("fragment", input);
  }

  toJSON(): string {
    return this.href;
  }

  toString(): string {
    return this.href;
  }

  get href(): string {
    return this[_urlRecord].toString();
  }

  set href(value: string) {
    // 1. Let parsedURL be the result of running the basic URL parser on the given value.
    // 2. If parsedURL is failure, then throw a TypeError.
    const parsedURL = new UrlRecord(value);
    // 3. Set this’s URL to parsedURL.
    this[_urlRecord] = parsedURL;
    // 4. Empty this’s query object’s list.
    this[_queryObject][_list] = [];
    // 5. Let query be this’s URL’s query.
    const query = parsedURL.get("query");
    if (query) {
      // 6. If query is non-null, then set this’s query object’s list to the result of parsing query.
      this[_queryObject][_list] = parse_url_encoded_form(query);
    }
  }
}

export { URLSearchParams, URL };
