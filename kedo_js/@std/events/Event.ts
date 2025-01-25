interface EventFlags {
  stopPropagation?: boolean;
  stopImmediatePropagation?: boolean;
  canceled?: boolean;
}

const _type = Symbol("[[type]]");
const _target = Symbol("[[Target]]");
const _currentTarget = Symbol("[[CurrentTarget]]");
const _flags = Symbol("[[Flags]]");
const _bubbles = Symbol("[[Bubbles]]");
const _cancelable = Symbol("[[Cancelable]]");
const _timeStamp = Symbol("[[TimeStamp]]");

class Event {
  [_type]: string;
  [_target]: EventTarget | null = null;
  [_currentTarget]: EventTarget | null = null;
  [_flags]: EventFlags;
  [_bubbles]: boolean;
  [_cancelable]: boolean;
  [_timeStamp]: number;

  constructor(type: string, eventInitDict?: EventInit) {
    this[_type] = type;
    this[_bubbles] = eventInitDict?.bubbles ?? false;
    this[_cancelable] = eventInitDict?.cancelable ?? false;
    this[_timeStamp] = Date.now();
    this[_flags] = {};
  }

  stopPropagation() {
    this[_flags].stopPropagation = true;
  }

  stopImmediatePropagation() {
    this[_flags].stopPropagation = true;
    this[_flags].stopImmediatePropagation = true;
  }

  preventDefault() {
    if (this[_cancelable]) {
      this[_flags].canceled = true;
    }
  }

  get type(): string {
    return this[_type];
  }

  get target(): EventTarget | null {
    return this[_target];
  }

  get currentTarget(): EventTarget | null {
    return this[_currentTarget];
  }

  get cancelable(): boolean {
    return this[_cancelable];
  }

  get bubbles(): boolean {
    return this[_bubbles];
  }

  get defaultPrevented(): boolean {
    return !!this[_flags].canceled;
  }

  get timeStamp(): number {
    return this[_timeStamp];
  }
}

class EventTarget {
  private listeners: Map<string, EventListener[]> = new Map();
  static uncaghtListernerException = "uncaughtListenerException";

  addEventListener(type: string, callback: EventListener) {
    if (!this.listeners.has(type)) {
      this.listeners.set(type, []);
    }
    this.listeners.get(type)?.push(callback);
  }

  removeEventListener(type: string, callback: EventListener) {
    const listeners = this.listeners.get(type);
    if (!listeners) return;

    const index = listeners.indexOf(callback);
    if (index !== -1) {
      listeners.splice(index, 1);
    }
  }

  dispatchEvent(event: Event): boolean {
    if (event[_flags].stopPropagation) {
      return false;
    }

    const listeners = this.listeners.get(event.type);
    if (!listeners) {
      if (event.type === EventTarget.uncaghtListernerException) {
        console.error(`Uncaught event listener exception`);
      }

      return true;
    }

    event[_target] = this;

    for (const listener of listeners) {
      try {
        listener(event);
      } catch (error) {
        if (event.type === EventTarget.uncaghtListernerException) {
          continue;
        }

        const errorEvent = new Event(EventTarget.uncaghtListernerException);
        event[_target] = this;
        this.dispatchEvent(errorEvent);
      }

      if (event[_flags].stopImmediatePropagation) {
        break; // Stop all event propagation and prevent other listeners on this target from being executed
      }
    }

    return !event.defaultPrevented;
  }
}

export { Event, EventTarget };
