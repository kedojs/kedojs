type Listener = (...args: any[]) => void | Promise<void>;

interface EventInit {
    bubbles?: boolean;
    cancelable?: boolean;
    composed?: boolean;
}

declare module "@kedo/events" {
    /**
     * Represents a DOM Event object that can be dispatched to event targets.
     * 
     * @class
     * @description Events are objects that provide information about an occurrence in the system,
     * such as a user interaction or lifecycle change.
     * 
     * @param {string} type - The name of the event
     * @param {EventInit} [eventInitDict] - Optional initialization parameters for the event
     * 
     * @property {string} type - The name/type of the event
     * @property {boolean} bubbles - Whether the event bubbles up through the DOM
     * @property {boolean} cancelable - Whether the event is cancelable
     * @property {boolean} defaultPrevented - Whether preventDefault() was called on the event
     * @property {number} eventPhase - The current phase of event propagation
     * @property {EventTarget | null} target - The object that dispatched the event
     * @property {EventTarget | null} currentTarget - The current target for the event
     * 
     * @method stopPropagation - Prevents further propagation of the current event
     * @method stopImmediatePropagation - Prevents other listeners of the same event from being called
     * @method preventDefault - Cancels the event if it is cancelable
     */
    class Event {
        constructor(type: string, eventInitDict?: EventInit);
        readonly type: string;
        readonly bubbles: boolean;
        readonly cancelable: boolean;
        readonly defaultPrevented: boolean;
        readonly eventPhase: number;
        readonly target: EventTarget | null;
        readonly currentTarget: EventTarget | null;
        stopPropagation(): void;
        stopImmediatePropagation(): void;
        preventDefault(): void;
    }

    /**
     * Represents an object that can receive events and may have listeners for them.
     * 
     * Implements the DOM EventTarget interface, providing methods to register and handle event listeners.
     * 
     * @class
     * @example
     * ```typescript
     * const target = new EventTarget();
     * target.addEventListener('click', (event) => {
     *   console.log('clicked');
     * });
     * ```
     */
    class EventTarget {
        addEventListener(type: string, listener: Listener): void;
        removeEventListener(type: string, listener: Listener): void;
        dispatchEvent(event: Event): boolean;
    }

    type EventListener = (event: Event) => void;
    /**
     * A class that implements the publish/subscribe pattern, allowing objects to subscribe to and emit events.
     * 
     * @class EventEmitter
     * 
     * @property {symbol} errorMonitor - Static symbol used for error monitoring
     * @property {string} errorEvent - Static string representing the error event name
     * 
     * @example
     * ```typescript
     * const emitter = new EventEmitter();
     * emitter.on('event', (data) => console.log(data));
     * emitter.emit('event', 'Hello World');
     * ```
     */
    class EventEmitter {
        static errorMonitor: symbol;
        static errorEvent: string;

        on(event: string | symbol, listener: Listener): EventEmitter;
        once(event: string | symbol, listener: Listener): EventEmitter;
        off(event: string | symbol, listener: Listener): EventEmitter;
        removeAllListeners(event?: string | symbol): void;
        emit(event: string | symbol, ...args: any[]): boolean;
        listenerCount(event: string | symbol): number;
        eventNames(): Array<string | symbol>;
        setMaxListeners(n: number): void;
        getMaxListeners(): number;
    }
}