/**
 * A set-like data structure that holds weak references to objects.
 *
 * This class is useful for keeping track of objects that should not prevent garbage collection.
 *
 * @typeparam T The type of elements in the set.
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
