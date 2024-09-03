// import assert from "node:assert";
import assert from "@kedo/assert";

function runTests() {
  // Test 1: Basic UTF-8 decoding
  (function testBasicUTF8Decoding() {
    const decoder = new TextDecoder();
    const input = new Uint8Array([0x48, 0x65, 0x6c, 0x6c, 0x6f]); // "Hello"
    const output = decoder.decode(input);
    assert.strictEqual(output, "Hello", "Basic UTF-8 decoding failed");
  })();

  // Test 2: UTF-8 decoding with BOM
  (function testUTF8DecodingWithBOM() {
    const decoder = new TextDecoder("utf-8", { ignoreBOM: true });
    const input = new Uint8Array([
      0xef, 0xbb, 0xbf, 0x48, 0x65, 0x6c, 0x6c, 0x6f,
    ]); // BOM + "Hello"
    const output = decoder.decode(input);
    assert.strictEqual(output, "\uFEFFHello", "UTF-8 decoding with BOM failed");
  })();

  // Test 3: UTF-8 decoding with BOM ignored
  (function testUTF8DecodingWithBOMIgnored() {
    const decoder = new TextDecoder("utf-8", { ignoreBOM: false });
    const input = new Uint8Array([
      0xef, 0xbb, 0xbf, 0x48, 0x65, 0x6c, 0x6c, 0x6f,
    ]); // BOM + "Hello"
    const output = decoder.decode(input);
    assert.strictEqual(
      output,
      "Hello",
      "UTF-8 decoding with BOM ignored failed",
    );
  })();

  // Test 4: UTF-8 decoding with invalid sequences (fatal mode)
  (function testUTF8DecodingWithInvalidSequencesFatal() {
    const decoder = new TextDecoder("utf-8", { fatal: true });
    const input = new Uint8Array([0xff, 0xff, 0xff]); // Invalid UTF-8
    //console.log(decoder.decode(input));
    assert.throws(
      () => decoder.decode(input),
      TypeError,
      "UTF-8 decoding with invalid sequences in fatal mode did not throw",
    );
  })();

  // Test 5: UTF-8 decoding with invalid sequences (replacement mode)
  (function testUTF8DecodingWithInvalidSequencesReplacement() {
    const decoder = new TextDecoder("utf-8", { fatal: false });
    const input = new Uint8Array([
      0xef, 0xbf, 0xbd, 0xef, 0xbf, 0xbd, 0xef, 0xbf, 0xbd,
    ]); // Invalid UTF-8
    const output = decoder.decode(input);
    assert.strictEqual(
      output,
      "\uFFFD\uFFFD\uFFFD",
      "UTF-8 decoding with invalid sequences failed",
    );
  })();

  // Test 6: Handling of empty input
  (function testEmptyInput() {
    const decoder = new TextDecoder();
    const output = decoder.decode(new Uint8Array([]));
    assert.strictEqual(output, "", "Decoding empty input failed");
  })();

  // Test 7: Handling of ArrayBuffer input
  (function testArrayBufferInput() {
    const decoder = new TextDecoder();
    const buffer = new Uint8Array([0x48, 0x65, 0x6c, 0x6c, 0x6f]).buffer; // "Hello"
    const output = decoder.decode(buffer);
    assert.strictEqual(output, "Hello", "Decoding ArrayBuffer input failed");
  })();

  // Test 8: Handling of DataView input
  (function testDataViewInput() {
    const decoder = new TextDecoder();
    const buffer = new Uint8Array([0x48, 0x65, 0x6c, 0x6c, 0x6f]).buffer; // "Hello"
    const dataView = new DataView(buffer);
    const output = decoder.decode(dataView);
    assert.strictEqual(output, "Hello", "Decoding DataView input failed");
  })();

  // Test 9: Stream decoding
  (function testStreamDecoding() {
    const decoder = new TextDecoder("utf-8", { fatal: false });
    const input1 = new Uint8Array([0x48, 0x65]); // "He"
    const input2 = new Uint8Array([0x6c, 0x6c, 0x6f]); // "llo"
    const output1 = decoder.decode(input1, { stream: true });
    const output2 = decoder.decode(input2, { stream: true });
    assert.strictEqual(output1, "He", "Stream decoding part 1 failed");
    assert.strictEqual(output2, "llo", "Stream decoding part 2 failed");
  })();

  // Test 8: Handling of DataView input
  (function testEncodingEncode() {
    const encoder = new TextEncoder();
    const decoder = new TextDecoder("utf-8", { fatal: false });
    const output = encoder.encode("Hello");
    assert.strictEqual(output[0], 0x48, "Encoding input failed");
    assert.strictEqual(output[1], 0x65, "Encoding input failed");
    assert.strictEqual(output[2], 0x6c, "Encoding input failed");
    assert.strictEqual(output[3], 0x6c, "Encoding input failed");
    assert.strictEqual(output[4], 0x6f, "Encoding input failed");
    assert.strictEqual(
      decoder.decode(output),
      "Hello",
      "Encoding input failed",
    );
  })();

  console.log("All tests passed!");
}

runTests();
