import { isDisturbed, ReadableStream } from "@kedo:int/std/stream";
import { TextEncoder } from "./TextDecoder";

const isReadableStream = (object: any): object is ReadableStream =>
    object instanceof ReadableStream;

const unusable = (stream: ReadableStream | null) => {
    if (stream === null) return false;
    return isDisturbed(stream as any) || stream.locked;
};

class InternalBody {
    private _body: ReadableStream | null;
    private _source?: Uint8Array;
    private _consumed?: boolean;

    constructor(body: ReadableStream | Uint8Array | null, consumed = false) {
        this._body = null;
        this._consumed = consumed;

        if (body instanceof Uint8Array) {
            this._source = body;
        } else {
            this._body = body;
        }
    }

    get body(): ReadableStream | null {
        if (this._source !== undefined) {
            const value = this._source;
            const stream = new ReadableStream({
                start(controller) {
                    controller.enqueue(value);
                    controller.close();
                },
            });
            this._source = undefined;
            this._body = stream;
        }

        return this._body;
    }

    get bodyUsed(): boolean {
        return this._body !== null && isDisturbed(this._body as any);
    }

    // Consume Body:
    // The Consume body function consist of converting the byte sequence into javascrip value
    // - 1. Check wheter the body is unsable by checking if it is different from null and stream is no disturbed or locked
    // - 2. If body is null, then return null
    // - 3. Fully Read the body:
    //     - 3.1. Start a parrallel bytes queue
    //     - 3.2. let reader be the result of acquiring a reader from body's stream
    //     - 3.3. read all the bytes from the reader and add them to the queue
    // - 4. resolve the prmise with the result of converting the queue into a javascript value
    private async consumeBody(): Promise<Uint8Array> {
        if (this._consumed || this.bodyUsed) {
            throw new TypeError("Body has already been consumed.");
        }

        if (this.body === null) {
            return new Uint8Array(0);
        }

        // 1. If object is unusable, then return a promise rejected with a TypeError.
        if (unusable(this.body)) {
            throw new TypeError("Body is unusable");
        }

        const reader = this.body.getReader<ReadableStreamDefaultReader>();
        this._consumed = true;

        const chunks: Uint8Array[] = [];
        let totalLength = 0;

        while (true) {
            // Allocate a new buffer (e.g., 1KB) for each read
            // TODO: Use a more efficient way to read the stream
            // const buffer = new Uint8Array(1024);
            const { value, done } = await reader.read();
            if (done) break;

            if (value && value.byteLength > 0) {
                totalLength += value.byteLength;
                chunks.push(value);
            }
        }

        // Combine all chunks into a single Uint8Array
        const result = new Uint8Array(totalLength);
        let offset = 0;

        for (const chunk of chunks) {
            result.set(chunk, offset);
            offset += chunk.byteLength;
        }

        return result;
    }

    async arrayBuffer(): Promise<ArrayBuffer> {
        const bytes = await this.consumeBody();
        return bytes.buffer.slice(
            bytes.byteOffset,
            bytes.byteOffset + bytes.byteLength,
        ) as ArrayBuffer;
    }

    async bytes(): Promise<Uint8Array> {
        return this.consumeBody();
    }

    async json(): Promise<any> {
        const text = await this.text();
        try {
            return JSON.parse(text);
        } catch (e) {
            throw new SyntaxError("Failed to parse JSON.");
        }
    }

    async text(): Promise<string> {
        const bytes = await this.consumeBody();
        return new TextDecoder("utf-8").decode(bytes);
    }
}

const encoder = new TextEncoder();

type BodyInit =
    | ArrayBuffer
    | ArrayBufferView
    | string
    | URLSearchParams
    | ReadableStream;

// TODO: this implementation must be optimized
function extractBody(
    object: BodyInit,
    keepalive = false,
): { body: ExtractedBody; type: string | null } {
    // Let stream be null.
    let stream: ReadableStream | null = null;
    let source: Uint8Array | null = null;
    let length: number | null = null;
    let type: string | null = null;

    if (isReadableStream(object)) {
        if (keepalive) {
            throw new TypeError(
                "ReadableStream cannot be used with keepalive set to true.",
            );
        }

        if (isDisturbed(object as any) || object.locked) {
            throw new TypeError("ReadableStream is unusable.");
        }

        stream = object;
    } else if (typeof object === "string") {
        // Scalar value string
        source = encoder.encode(object);
        type = "text/plain;charset=UTF-8";
    } else if (object instanceof ArrayBuffer) {
        source = new Uint8Array(object).slice();
        length = object.byteLength;
        type = "application/octet-stream";
    } else if (ArrayBuffer.isView(object)) {
        source = new Uint8Array(
            object.buffer,
            object.byteOffset,
            object.byteLength,
        ).slice();
        length = object.byteLength;
        type = "application/octet-stream";
    } else if (object instanceof URLSearchParams) {
        // URLSearchParams
        source = encoder.encode(object.toString());
        type = "application/x-www-form-urlencoded;charset=UTF-8";
    } else {
        throw new TypeError("Invalid body type");
    }

    const body: ExtractedBody = { stream, source, length, type };
    return { body, type };
}

const mixinBody = (
    input: MixingBodyInput,
    _bodyKey: keyof MixingBodyInput,
    consumed = false,
) => {
    const body = input[_bodyKey] as any as ExtractedBody | null;
    const innerBody = new InternalBody(
        body?.stream || body?.source || null,
        consumed,
    );

    const mixin = {
        body: {
            __proto__: null,
            get(): ReadableStream | null {
                return innerBody.body;
            },
            configurable: true,
            enumerable: true,
        },
        bodyUsed: {
            __proto__: null,
            get(): boolean {
                return innerBody.bodyUsed;
            },
        },
        arrayBuffer: {
            __proto__: null,
            value: function arrayBuffer(): Promise<ArrayBuffer> {
                return innerBody.arrayBuffer();
            },
            writable: true,
            configurable: true,
            enumerable: true,
        },
        bytes: {
            __proto__: null,
            value: function bytes(): Promise<Uint8Array> {
                return innerBody.bytes();
            },
            writable: true,
            configurable: true,
            enumerable: true,
        },
        json: {
            __proto__: null,
            value: function json(): Promise<any> {
                return innerBody.json();
            },
            writable: true,
            configurable: true,
            enumerable: true,
        },
        text: {
            __proto__: null,
            value: function text(): Promise<string> {
                return innerBody.text();
            },
            writable: true,
            configurable: true,
            enumerable: true,
        },
    };

    Object.defineProperties(input, mixin);
};

export { extractBody, mixinBody };
