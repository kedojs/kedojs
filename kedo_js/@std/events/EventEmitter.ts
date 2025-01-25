// |<------------------->|<------------------->|<--------------------->|
// |                           EventEmitter                            |
// |-------------------------------------------------------------------|
//
// The EventEmitter class is a simple implementation of the EventEmitter pattern.
// It allows you to subscribe to events and emit them.
//
// ## Usage
//  ```ts
// import { EventEmitter } from "@kedo/events";
//
// const emitter = new EventEmitter();
//
// emitter.on("event", (arg1, arg2) => {
//  console.log("event", arg1, arg2);
// });
//
// emitter.emit("event", "arg1", "arg2");
class EventEmitter {
  private events: Map<string | symbol, Listener[]> = new Map();
  private maxListeners: number = 10;
  static errorMonitor = Symbol("events.errorMonitor");
  static errorEvent: string = "error";

  /**
   * Subscribe to an event
   *
   * @param event
   * @param listener
   * @returns
   */
  on(event: string | symbol, listener: Listener): EventEmitter {
    if (!this.events.has(event)) {
      this.events.set(event, []);
    }

    this.events.get(event)!.push(listener);
    this.checkMaxListeners(event);
    return this;
  }

  /**
   * Subscribe to an event only once
   *
   * @param event
   * @param listener
   * @returns
   */
  once(event: string | symbol, listener: Listener): EventEmitter {
    const onceWrapper = (...args: any[]) => {
      this.off(event, onceWrapper);
      listener.apply(this, args);
    };

    this.on(event, onceWrapper);
    return this;
  }

  /**
   * Unsubscribe from an event
   *
   * @param event
   * @param listener
   * @returns
   */
  off(event: string | symbol, listener: Listener): EventEmitter {
    if (!this.events.has(event)) return this;

    const listeners = this.events.get(event)!;
    const index = listeners.indexOf(listener);
    if (index !== -1) {
      listeners.splice(index, 1);
    }

    if (listeners.length === 0) {
      this.events.delete(event);
    }

    return this;
  }

  /**
   * Remove all listeners for an event
   * or all listeners for all events if no event is provided
   *
   * @param event
   */
  removeAllListeners(event?: string | symbol) {
    if (event) {
      this.events.delete(event);
    } else {
      this.events.clear();
    }
  }

  private handleErrorEvent(event: string | symbol, err: unknown) {
    if (
      event === EventEmitter.errorEvent ||
      event === EventEmitter.errorMonitor
    ) {
      throw err;
    }

    if (this.events.has(EventEmitter.errorMonitor)) {
      this.emit(EventEmitter.errorMonitor, err);
    }

    this.emit(EventEmitter.errorEvent, err);
  }

  /**
   * Emit an event
   * Returns true if the event had listeners, false otherwise
   *
   * @param event
   * @param args
   * @returns
   */
  emit(event: string | symbol, ...args: any[]): boolean {
    if (!this.events.has(event)) {
      if (event === EventEmitter.errorEvent) {
        throw args.length > 0 ? args[0] : new Error("Unhandled error event");
      }

      return false;
    }

    const listeners = this.events.get(event)!.slice();
    for (const listener of listeners) {
      try {
        const result = listener.apply(this, args);
        if (result instanceof Promise) {
          result.catch((err) => this.handleErrorEvent(event, err));
        }
      } catch (err) {
        this.handleErrorEvent(event, err);
      }
    }

    return true;
  }

  listenerCount(event: string | symbol): number {
    return this.events.has(event) ? this.events.get(event)!.length : 0;
  }

  eventNames(): (string | symbol)[] {
    return Array.from(this.events.keys());
  }

  setMaxListeners(n: number) {
    this.maxListeners = n;
  }

  getMaxListeners(): number {
    return this.maxListeners;
  }

  private checkMaxListeners(event: string | symbol): void {
    const listeners = this.listenerCount(event);
    if (listeners > this.maxListeners && this.maxListeners > 0) {
      console.warn(
        `MaxListenersExceededWarning: Possible EventEmitter memory leak detected. ${listeners} ${String(event)} listeners added. Use emitter.setMaxListeners() to increase limit`,
      );
    }
  }
}

export { EventEmitter };
