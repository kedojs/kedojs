// import assert from "node:assert";
import assert from "@kedo/assert";

// Test: Initialization with string
let params = new URLSearchParams("key1=value1&key2=value2");
assert.deepStrictEqual(
  params.toString(),
  "key1=value1&key2=value2",
  "Initialization with string",
);

// Test: Initialization with array
params = new URLSearchParams([
  ["key1", "value1"],
  ["key2", "value2"],
]);
assert.deepStrictEqual(
  params.toString(),
  "key1=value1&key2=value2",
  "Initialization with array",
);

// Test: Initialization with record
params = new URLSearchParams({ key1: "value1", key2: "value2" });
assert.deepStrictEqual(
  params.toString(),
  "key1=value1&key2=value2",
  "Initialization with record",
);

// Test: Append method
params = new URLSearchParams();
params.append("key1", "value1");
params.append("key1", "value2");
assert.deepStrictEqual(
  params.getAll("key1"),
  ["value1", "value2"],
  "Append method",
);

// Test: Delete method
params = new URLSearchParams("key1=value1&key2=value2&key1=value3");
params.delete("key1");
assert.deepStrictEqual(params.toString(), "key2=value2", "Delete method");

// Test: Get method
params = new URLSearchParams("key1=value1&key2=value2");
assert.strictEqual(params.get("key1"), "value1", "Get method");

// Test: GetAll method
params = new URLSearchParams("key1=value1&key1=value2");
assert.deepStrictEqual(
  params.getAll("key1"),
  ["value1", "value2"],
  "GetAll method",
);

// Test: Has method
params = new URLSearchParams("key1=value1&key2=value2");
assert.strictEqual(
  params.has("key1"),
  true,
  "Has method should return true for existing key",
);
assert.strictEqual(
  params.has("key3"),
  false,
  "Has method should return false for non-existing key",
);

// Test: Set method
params = new URLSearchParams("key1=value1&key2=value2");
params.set("key1", "newvalue");
assert.strictEqual(
  params.get("key1"),
  "newvalue",
  "Set method should update existing key",
);

// Test: Sort method
params = new URLSearchParams("z=value1&y=value2&x=value3");
params.sort();
assert.deepStrictEqual(
  params.toString(),
  "x=value3&y=value2&z=value1",
  "Sort method",
);

// Test: Iterator
params = new URLSearchParams("key1=value1&key2=value2");
const entries = [];
for (const entry of params) {
  entries.push(entry);
}
assert.deepStrictEqual(
  entries,
  [
    ["key1", "value1"],
    ["key2", "value2"],
  ],
  "Iterator",
);

const paramsString = "q=URLUtils.searchParams&topic=api";
const searchParams = new URLSearchParams(paramsString);

// Iterating the search parameters
const _temp_values = [];
for (const p of searchParams) {
  _temp_values.push(p);
}
assert.deepStrictEqual(
  _temp_values,
  [
    ["q", "URLUtils.searchParams"],
    ["topic", "api"],
  ],
  "Iterating the search parameters",
);

assert.ok(searchParams.has("topic")); // true
// assert.ok(!searchParams.has("topic", "fish")); // false
assert.ok(searchParams.get("topic") === "api"); // true
assert.deepStrictEqual(
  searchParams.getAll("topic"),
  ["api"],
  "Get all values for a key",
); // ["api"]
assert.ok(searchParams.get("foo") === null); // true
searchParams.append("topic", "webdev");
assert.strictEqual(
  searchParams.toString(),
  "q=URLUtils.searchParams&topic=api&topic=webdev",
  "Append a new value to the existing key",
); // "q=URLUtils.searchParams&topic=api&topic=webdev"
searchParams.set("topic", "More webdev");
assert.strictEqual(
  searchParams.toString(),
  "q=URLUtils.searchParams&topic=More+webdev",
  "Set a new value to the existing key",
); // "q=URLUtils.searchParams&topic=More+webdev"
searchParams.delete("topic");
assert.strictEqual(
  searchParams.toString(),
  "q=URLUtils.searchParams",
  "Delete an existing key",
); // "q=URLUtils.searchParams"

// Search parameters can also be an object
const paramsObj = { foo: "bar", baz: "bar" };
const searchParams2 = new URLSearchParams(paramsObj);

assert.strictEqual(
  searchParams2.toString(),
  "foo=bar&baz=bar",
  "Search parameters can also be an object",
); // "foo=bar&baz=bar"
assert.ok(searchParams2.has("foo")); // true
assert.strictEqual(searchParams2.get("foo"), "bar", "Get the value of a key"); // "bar"

const paramStr = "foo=bar&foo=baz";
const searchParams3 = new URLSearchParams(paramStr);

assert.strictEqual(
  searchParams3.toString(),
  "foo=bar&foo=baz",
  "Search parameters can also be a string",
); // "foo=bar&foo=baz"
assert.ok(searchParams3.has("foo")); // true
assert.strictEqual(
  searchParams3.get("foo"),
  "bar",
  "Get the first value of a key",
); // bar, only returns the first value
assert.deepStrictEqual(
  searchParams3.getAll("foo"),
  ["bar", "baz"],
  "Get all values for a key",
); // ["bar", "baz"]

// No URL parsing
const paramsString1 = "http://example.com/search?query=%40";
const searchParams1 = new URLSearchParams(paramsString1);

assert.ok(!searchParams1.has("query")); // false
assert.ok(searchParams1.has("http://example.com/search?query")); // true

assert.strictEqual(searchParams1.get("query"), null, "Get the value of a key"); // null
assert.strictEqual(
  searchParams1.get("http://example.com/search?query"),
  "@",
  "Get the value of a key",
); // "@" (equivalent to decodeURIComponent('%40'))

const paramsString2 = "?query=value";
const searchParams4 = new URLSearchParams(paramsString2);
assert.ok(searchParams4.has("query")); // true

const url = new URL("http://example.com/search?query=%40");
const searchParams5 = new URLSearchParams(url.search);
assert.ok(searchParams5.has("query")); // true

const emptyVal = new URLSearchParams("foo=&bar=baz");
assert.strictEqual(emptyVal.get("foo"), ""); // returns ''
const noEquals = new URLSearchParams("foo&bar=baz");
assert.strictEqual(noEquals.get("foo"), ""); // also returns ''
assert.strictEqual(noEquals.toString(), "foo=&bar=baz"); // 'foo=&bar=baz'

const url2 = new URL("https://example.com/?a=hello&b=world");
assert.strictEqual(url2.href, "https://example.com/?a=hello&b=world");
assert.strictEqual(url2.origin, "https://example.com");
const add_params = {
  c: "a",
  d: new String(2),
  e: false.toString(),
};

const new_params = new URLSearchParams([
  ...Array.from(url2.searchParams.entries()), // [["a","hello"],["b","world"]]
  ...Object.entries(add_params), // [["c","a"],["d","2"],["e","false"]]
]).toString();
assert.strictEqual(new_params, "a=hello&b=world&c=a&d=2&e=false");
// a=hello&b=world&c=a&d=2&e=false
const new_url = new URL(`${url2.origin}${url2.pathname}?${new_params}`);
assert.strictEqual(
  new_url.href,
  "https://example.com/?a=hello&b=world&c=a&d=2&e=false",
);
// https://example.com/?a=hello&b=world&c=a&d=2&e=false

const url5 = new URL("../cats", "http://www.example.com/dogs");
assert.strictEqual(url5.hostname, "www.example.com"); // "www.example.com"
assert.strictEqual(url5.pathname, "/cats"); // "/cats"

assert.ok(URL.canParse("../cats", "http://www.example.com/dogs"));

url5.hash = "tabby";
assert.strictEqual(url5.href, "http://www.example.com/cats#tabby"); // "http://www.example.com/cats#tabby"

url5.pathname = "démonstration.html";
// http://www.example.com/d%C3%A9monstration.htm#tabby
assert.strictEqual(
  url5.href,
  "http://www.example.com/d%C3%A9monstration.html#tabby",
); // "http://www.example.com/d%C3%A9monstration.html"

// https://some.site/?id=123
const parsedUrl = new URL("https://some.site/?id=123");
assert.strictEqual(parsedUrl.searchParams.get("id"), "123"); // "123"

// |-----------------|
// | URL Test Cases  |
// |-----------------|

// Absolute URL
let result = URL.parse("https://developer.mozilla.org/en-US/docs");
assert.strictEqual(result.href, "https://developer.mozilla.org/en-US/docs");

// Relative reference to a valid base URL
result = URL.parse("en-US/docs", "https://developer.mozilla.org");
assert.strictEqual(result.href, "https://developer.mozilla.org/en-US/docs");

// Relative reference to a "complicated" valid base URL
// (only the scheme and domain are used to resolve url)
result = URL.parse(
  "/different/place",
  "https://developer.mozilla.org:443/some/path?id=4",
);
assert.strictEqual(
  result.href,
  "https://developer.mozilla.org/different/place",
);

// Absolute url argument (base URL ignored)
result = URL.parse(
  "https://example.org/some/docs",
  "https://developer.mozilla.org",
);
assert.strictEqual(result.href, "https://example.org/some/docs");

// Invalid base URL (missing colon)
result = URL.parse("en-US/docs", "https//developer.mozilla.org");
assert.strictEqual(result, null);

result = URL.parse("/en-US/docs", new URL("https://developer.mozilla.org/"));
assert.strictEqual(result.href, "https://developer.mozilla.org/en-US/docs");

assert.ok(!URL.canParse("en-US/docs", "https//developer.mozilla.org"));
assert.ok(
  URL.canParse(
    "https://example.org/some/docs",
    "https://developer.mozilla.org",
  ),
);

assert.strictEqual(
  new URL(
    "https://developer.mozilla.org/en-US/docs/Web/API/URL/toString",
  ).toString(),
  "https://developer.mozilla.org/en-US/docs/Web/API/URL/toString",
);

assert.strictEqual(
  new URL(
    "https://developer.mozilla.org/en-US/docs/Web/API/URL/toJSON",
  ).toJSON(),
  "https://developer.mozilla.org/en-US/docs/Web/API/URL/toJSON",
);

const url6 = new URL(
  "https://developer.mozilla.org/en-US/docs/Web/API/URL/href#examples",
);
assert.strictEqual(url6.hash, "#examples");

// host
let url7 = new URL("https://developer.mozilla.org/en-US/docs/Web/API/URL/host");
assert.strictEqual(url7.host, "developer.mozilla.org"); // "developer.mozilla.org"

url7 = new URL("https://developer.mozilla.org:443/en-US/docs/Web/API/URL/host");
assert.strictEqual(url7.host, "developer.mozilla.org"); // "developer.mozilla.org"
// The port number is not included because 443 is the scheme's default port

url7 = new URL(
  "https://developer.mozilla.org:4097/en-US/docs/Web/API/URL/host",
);
assert.strictEqual(url7.host, "developer.mozilla.org:4097"); // "developer.mozilla.org:4097"

const url8 = new URL(
  "https://developer.mozilla.org/en-US/docs/Web/API/URL/hostname",
);
assert.strictEqual(url8.hostname, "developer.mozilla.org"); // Logs: 'developer.mozilla.org'

const url9 = new URL("blob:https://mozilla.org:443/");
assert.strictEqual(url9.origin, "https://mozilla.org"); // 'https://mozilla.org'

const url10 = new URL("http://localhost:80/");
assert.strictEqual(url10.origin, "http://localhost"); // 'http://localhost'

const url11 = new URL("https://mozilla.org:8080/");
assert.strictEqual(url11.origin, "https://mozilla.org:8080"); // 'https://mozilla.org:8080'

const url12 = new URL(
  "https://anonymous:flabada@developer.mozilla.org/en-US/docs/Web/API/URL/password",
);
assert.strictEqual(url12.password, "flabada"); // Logs "flabada"

const url13 = new URL(
  "https://developer.mozilla.org/en-US/docs/Web/API/URL/pathname?q=value",
);
assert.strictEqual(url13.pathname, "/en-US/docs/Web/API/URL/pathname"); // Logs "/en-US/docs/Web/API/URL/pathname"

// https protocol with non-default port number
assert.strictEqual(new URL("https://example.com:5443/svn/Repos/").port, "5443"); // '5443'
// http protocol with non-default port number
assert.strictEqual(new URL("http://example.com:8080/svn/Repos/").port, "8080"); // '8080'
// https protocol with default port number
assert.strictEqual(new URL("https://example.com:443/svn/Repos/").port, ""); // '' (empty string)
// http protocol with default port number
assert.strictEqual(new URL("http://example.com:80/svn/Repos/").port, ""); // '' (empty string)
// https protocol with no explicit port number
assert.strictEqual(new URL("https://example.com/svn/Repos/").port, ""); // '' (empty string)
// http protocol with no explicit port number
assert.strictEqual(new URL("https://example.com/svn/Repos/").port, ""); // '' (empty string)
// ftp protocol with non-default port number
assert.strictEqual(new URL("ftp://example.com:221/svn/Repos/").port, "221"); // '221'
// ftp protocol with default port number
assert.strictEqual(new URL("ftp://example.com:21/svn/Repos/").port, ""); // '' (empty string)

assert.strictEqual(
  new URL("https://example.com:5443/svn/Repos/").protocol,
  "https:",
); // 'https:'
assert.strictEqual(
  new URL("http://example.com:8080/svn/Repos/").protocol,
  "http:",
); // 'http:'
assert.strictEqual(
  new URL("ftp://example.com:221/svn/Repos/").protocol,
  "ftp:",
); // 'ftp:'

const url14 = new URL(
  "https://developer.mozilla.org/en-US/docs/Web/API/URL/search?q=123",
);
assert.strictEqual(url14.search, "?q=123"); // Logs "?q=123"

const url15 = new URL(
  "https://anonymous:flabada@developer.mozilla.org/en-US/docs/Web/API/URL/username",
);
assert.strictEqual(url15.username, "anonymous"); // Logs "anonymous"

const params_v1 = new URL("https://example.com/?name=Jonathan%20Smith&age=18")
  .searchParams;
const name = params_v1.get("name");
const age = parseInt(params_v1.get("age"));

assert.strictEqual(name, "Jonathan Smith"); // name: Jonathan Smith
assert.strictEqual(age, 18); // age: 18

// |----------------------------------------|
// | Test: Initialization with absolute URL |
// |----------------------------------------|
let url20 = new URL("https://example.com:8080/path?query=123#fragment");
assert.strictEqual(
  url20.href,
  "https://example.com:8080/path?query=123#fragment",
  "Initialization with absolute URL",
);

// Test: Initialization with relative URL
url20 = new URL("/path", "https://example.com");
assert.strictEqual(
  url20.href,
  "https://example.com/path",
  "Initialization with relative URL",
);

// Test: Protocol accessor
url20 = new URL("https://example.com");
assert.strictEqual(url20.protocol, "https:", "Protocol getter");
url20.protocol = "http:";
assert.strictEqual(url20.protocol, "http:", "Protocol setter");
assert.strictEqual(
  url20.href,
  "http://example.com/",
  "Protocol setter effect on href",
);

// Test: Username and Password accessors
url20 = new URL("https://username:password@example.com");
assert.strictEqual(url20.username, "username", "Username getter");
assert.strictEqual(url20.password, "password", "Password getter");
url20.username = "newuser";
url20.password = "newpass";
assert.strictEqual(url20.username, "newuser", "Username setter");
assert.strictEqual(url20.password, "newpass", "Password setter");
assert.strictEqual(
  url20.href,
  "https://newuser:newpass@example.com/",
  "Username and Password setters effect on href",
);

// Test: Host, Hostname, and Port accessors
url20 = new URL("https://example.com:8080");
assert.strictEqual(url20.host, "example.com:8080", "Host getter");
assert.strictEqual(url20.hostname, "example.com", "Hostname getter");
assert.strictEqual(url20.port, "8080", "Port getter");
url20.host = "newexample.com:9090";
assert.strictEqual(url20.host, "newexample.com:9090", "Host setter");
assert.strictEqual(
  url20.hostname,
  "newexample.com",
  "Hostname setter via host",
);
assert.strictEqual(url20.port, "9090", "Port setter via host");

// Test: Pathname accessor
url20 = new URL("https://example.com/path");
assert.strictEqual(url20.pathname, "/path", "Pathname getter");
url20.pathname = "/newpath";
assert.strictEqual(url20.pathname, "/newpath", "Pathname setter");
assert.strictEqual(
  url20.href,
  "https://example.com/newpath",
  "Pathname setter effect on href",
);

// Test: Search accessor
url20 = new URL("https://example.com?query=123");
assert.strictEqual(url20.search, "?query=123", "Search getter");
url20.search = "?newquery=456";
assert.strictEqual(url20.search, "?newquery=456", "Search setter");
assert.strictEqual(
  url20.href,
  "https://example.com/?newquery=456",
  "Search setter effect on href",
);

// Test: Hash accessor
url20 = new URL("https://example.com#fragment");
assert.strictEqual(url20.hash, "#fragment", "Hash getter");
url20.hash = "#newfragment";
assert.strictEqual(url20.hash, "#newfragment", "Hash setter");
assert.strictEqual(
  url20.href,
  "https://example.com/#newfragment",
  "Hash setter effect on href",
);

// Test: Origin accessor
url20 = new URL("https://example.com:8080/path?query=123#fragment");
assert.strictEqual(url20.origin, "https://example.com:8080", "Origin getter");

// Test: URLSearchParams integration
url20 = new URL("https://example.com?key1=value1&key2=value2");
assert.deepStrictEqual(
  url20.searchParams.get("key1"),
  "value1",
  "URLSearchParams integration - get",
);
url20.searchParams.append("key3", "value3");
assert.strictEqual(
  url20.search,
  "?key1=value1&key2=value2&key3=value3",
  "URLSearchParams integration - append",
);
url20.searchParams.delete("key1");
assert.strictEqual(
  url20.search,
  "?key2=value2&key3=value3",
  "URLSearchParams integration - delete",
);

// Test: Static parse method
const parsedURL = URL.parse("https://example.com/path");
assert.strictEqual(
  parsedURL?.href,
  "https://example.com/path",
  "Static parse method",
);

// Test: Static canParse method
assert.strictEqual(
  URL.canParse("https://example.com/path"),
  true,
  "Static canParse method - valid URL",
);
assert.strictEqual(
  URL.canParse("invalid url"),
  false,
  "Static canParse method - invalid URL",
);

console.log("All tests passed");
