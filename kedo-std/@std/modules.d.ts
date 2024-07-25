declare module '@kedo/internal/utils' {
    export function is_array_buffer_detached(buffer: ArrayBufferLike): boolean;
}

declare module '@kedo/ds' {
    class Queue<T> {
        constructor();
        enqueue(value: T): void;
        dequeue(): T;
        peek(): T;
        isEmpty(): boolean;
        clear(): void;
        get size(): number;
    }
}