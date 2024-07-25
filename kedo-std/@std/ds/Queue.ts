
class Node<V> {
    value: V;
    next: Node<V> | null;

    constructor(value: V) {
        this.value = value;
        this.next = null;
    }
}

class Queue<V> {
    #head: Node<V> | null;
    #tail: Node<V> | null;
    #length: number;

    constructor() {
        this.#head = null; // The front of the queue
        this.#tail = null; // The end of the queue
        this.#length = 0;  // The number of elements in the queue
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
            throw new Error('Queue is empty');
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
            throw new Error('Queue is empty');
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
