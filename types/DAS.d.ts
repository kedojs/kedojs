declare module "@kedo/ds" {

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
    class Queue<T> {
        constructor();
        enqueue(value: T): void;
        dequeue(): T;
        peek(): T;
        isEmpty(): boolean;
        clear(): void;
        get size(): number;
    }

    /**
     * @class
     * A set-like collection of objects that holds weak references to its elements. 
     * This allows objects to be garbage-collected if there are no other strong references 
     * to them, while still enabling iteration over existing elements.
     * 
     * @template T - The type of objects stored in the set. Must extend `object`.
     * 
     * @remarks
     * Weak references do not prevent their referents from being reclaimed by the garbage collector.
     * Therefore, any element in this set may vanish at any time if no other strong references exist.
     * This class provides iterative functionality, but be aware that the iteration may skip elements
     * that have been garbage-collected.
     * 
     * @constructor
     * Creates a new empty IterableWeakSet.
     * 
     * @method add
     * Adds a new object to the set.
     * @param value - The object to be added.
     * @returns The instance of the set for chaining.
     * 
     * @method delete
     * Removes an object from the set.
     * @param value - The object to remove.
     * @returns A boolean indicating whether the object was found and removed.
     * 
     * @method has
     * Checks if an object is present in the set.
     * @param value - The object to check for presence.
     * @returns A boolean indicating whether the object is in the set.
     * 
     * @method [Symbol.iterator]
     * Returns an iterator that yields each valid (non-garbage-collected) object in the set.
     * @returns An iterator over the objects in the set.
     * 
     * @method clear
     * Clears all objects from the set.
     * 
     * @method size
     * Returns the current number of objects in the set that have not been garbage-collected.
     * @returns The number of objects actively referenced.
     */
    class IterableWeakSet<T extends object> {
        constructor();
        add(value: T): IterableWeakSet<T>;
        delete(value: T): boolean;
        has(value: T): boolean;
        [Symbol.iterator](): Iterator<T>;
        clear(): void;
        size(): number;
    }
}
