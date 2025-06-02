class Node<V> {
    value: V;
    next: Node<V> | null;

    constructor(value: V) {
        this.value = value;
        this.next = null;
    }
}

/**
 * A generic queue data structure that provides methods to manage elements in a FIFO manner.
 *
 * @typeParam T - The type of elements held in the queue.
 *
 * @remarks
 * - Use the enqueue method to add elements to the end of the queue.
 * - Use the dequeue method to remove and retrieve the first element.
 * - Use the peek method to check the first element without removing it.
 * - Use the isEmpty method to check whether the queue contains any elements.
 * - Use the clear method to remove all elements at once.
 *
 * @example
 * ```ts
 * const queue = new Queue<number>();
 * queue.enqueue(1);
 * queue.enqueue(2);
 * console.log(queue.dequeue()); // 1
 * console.log(queue.peek());    // 2
 * ```
 *
 * @public
 * @constructor
 * Creates a new empty Queue.
 *
 * @method enqueue
 * Adds a new element to the end of the queue.
 * @param value - The element to be added.
 *
 * @method dequeue
 * Removes and returns the first element in the queue.
 * @returns The first element in the queue.
 *
 * @method peek
 * Returns the first element in the queue without removing it.
 * @returns The first element in the queue.
 *
 * @method isEmpty
 * Checks if the queue is empty.
 * @returns A boolean indicating whether the queue is empty.
 *
 * @method clear
 * Removes all elements from the queue.
 *
 * @property size
 * Returns the current number of elements in the queue.
 */
class Queue<V> {
    #head: Node<V> | null;
    #tail: Node<V> | null;
    #length: number;

    constructor() {
        this.#head = null; // The front of the queue
        this.#tail = null; // The end of the queue
        this.#length = 0; // The number of elements in the queue
    }

    // Adds an element to the back of the queue
    enqueue(value: V) {
        const newNode = new Node(value);
        if (this.#tail) {
            this.#tail.next = newNode;
        }

        this.#tail = newNode;
        if (!this.#head) {
            this.#head = newNode;
        }

        this.#length++;
    }

    // Removes an element from the front of the queue
    dequeue() {
        if (this.isEmpty()) {
            throw new Error("Queue is empty");
        }

        const value = this.#head!.value;
        this.#head = this.#head!.next;
        if (!this.#head) {
            this.#tail = null;
        }

        this.#length--;
        return value;
    }

    // Returns the element at the front of the queue without removing it
    peek(): V {
        if (this.isEmpty()) {
            throw new Error("Queue is empty");
        }

        return this.#head!.value;
    }

    // Returns the number of elements in the queue
    get size() {
        return this.#length;
    }

    // Returns true if the queue is empty, false otherwise
    isEmpty() {
        return this.#length === 0;
    }

    // Removes all elements from the queue
    clear() {
        this.#head = null;
        this.#tail = null;
        this.#length = 0;
    }
}

export { Queue };
