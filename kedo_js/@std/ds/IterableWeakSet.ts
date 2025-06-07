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
    private weakSet: WeakSet<T>;
    private refList: Set<WeakRef<T>>;

    constructor() {
        this.weakSet = new WeakSet<T>();
        this.refList = new Set<WeakRef<T>>();
    }

    add(value: T): IterableWeakSet<T> {
        if (!this.weakSet.has(value)) {
            this.weakSet.add(value);
            this.refList.add(new WeakRef(value));
        }

        return this;
    }

    delete(value: T): boolean {
        const existed = this.weakSet.has(value);
        if (existed) {
            this.weakSet.delete(value);
            // Manually remove the corresponding WeakRef from refList
            for (const ref of this.refList) {
                if (ref.deref() === value) {
                    this.refList.delete(ref);
                    break;
                }
            }
        }

        return existed;
    }

    has(value: T): boolean {
        return this.weakSet.has(value);
    }

    *[Symbol.iterator](): Iterator<T> {
        for (const ref of this.refList) {
            const item = ref.deref();
            if (item !== undefined) {
                yield item;
            }
        }
    }

    clear() {
        this.weakSet = new WeakSet<T>();
        this.refList.clear();
    }

    size(): number {
        this.cleanUpStaleRefs(); // Ensure size is calculated accurately
        return this.refList.size;
    }

    private cleanUpStaleRefs() {
        for (const ref of this.refList) {
            if (ref.deref() === undefined) {
                this.refList.delete(ref);
            }
        }
    }
}

export { IterableWeakSet };
