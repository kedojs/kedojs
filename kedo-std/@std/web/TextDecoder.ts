import {
  encoding_for_label_no_replacement,
  encoding_decode_utf8_once,
  encoding_decode,
  encoding_decode_once,
  EncodingTextDecoder,
  encoding_encode,
} from "@kedo/internal/utils";
import { isDataView, isTypedArray } from "../utils";

interface TextDecoderOptions {
  fatal?: boolean;
  ignoreBOM?: boolean;
}

interface TextDecodeOptions {
  stream?: boolean;
}

type TextDecodeInput = ArrayBufferView | ArrayBuffer | DataView;

class TextDecoder {
  #encoding: string;
  #ignoreBOM: boolean;
  private decoder: EncodingTextDecoder | null = null;
  // private ignoreBOMSet: boolean = false;
  private errorMode: "fatal" | "replacement" = "replacement";
  private doNotFlush: boolean = false;

  constructor(label: string = "utf-8", options?: TextDecoderOptions) {
    let encoding;
    try {
      encoding = encoding_for_label_no_replacement(label);
    } catch (e: any) {
      throw new RangeError(e.message);
    }

    this.#encoding = encoding;
    // If options["fatal"] is true, then set this’s error mode to "fatal".
    this.errorMode = options?.fatal ? "fatal" : "replacement";
    // Set this’s ignore BOM to options["ignoreBOM"].
    this.#ignoreBOM = options?.ignoreBOM ?? false;
  }

  get encoding(): string {
    return this.#encoding;
  }

  get fatal(): boolean {
    return this.errorMode === "fatal";
  }

  get ignoreBOM(): boolean {
    return this.#ignoreBOM;
  }

  decode(input?: TextDecodeInput, options?: TextDecodeOptions): string {
    // 1. If input is not given, set input to a new Uint8Array object whose [[ArrayBuffer]] is a new ArrayBuffer object containing a single 0 byte.
    if (input === undefined) {
      input = new Uint8Array(new ArrayBuffer(1));
    }

    // 2. If options is given and options["stream"] is true, then set stream to true.
    const stream = options?.stream ?? false;
    this.doNotFlush = stream;
    try {
      let buffer: ArrayBuffer = input as ArrayBuffer;
      if (isTypedArray(input)) {
        buffer = input.buffer;
      } else if (isDataView(input)) {
        buffer = input.buffer;
      }

      if (!stream && this.decoder === null) {
        if (this.encoding === "utf-8" && !this.fatal) {
          return encoding_decode_utf8_once(buffer, this.#ignoreBOM);
        }

        return encoding_decode_once(
          buffer,
          this.encoding,
          this.fatal,
          this.#ignoreBOM,
        );
      }

      if (this.decoder === null) {
        this.decoder = new EncodingTextDecoder(
          this.encoding,
          this.fatal,
          this.#ignoreBOM,
        );
      }

      return encoding_decode(this.decoder, buffer, stream);
    } finally {
      if (!stream && this.decoder !== null) {
        this.decoder = null;
      }
    }
  }
}

class TextEncoder {
  constructor() {}

  get encoding(): string {
    return "utf-8";
  }

  encode(input = ""): Uint8Array {
    return encoding_encode(input);
  }
}

export { TextDecoder, TextEncoder };
