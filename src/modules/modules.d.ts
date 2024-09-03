declare module "@kedo/internal/utils" {
  export function is_array_buffer_detached(buffer: ArrayBufferLike): boolean;
  export function parse_url_encoded_form(body: string): [string, string][];
  export class UrlRecord {
    constructor(url: string, base?: string);
    get(key: string): string | null;
    set(key: string, value: string): void;
    toString(): string;
  }
}
