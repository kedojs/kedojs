import { assert } from "@kedo/utils";

// TODO: this implemention is not being used.
// | -------------------------------------------- |
// | https://dom.spec.whatwg.org/#interface-event |
// |                    Event                     |
// | -------------------------------------------- |

interface EventFlags {
  stopPropagation?: boolean;
  stopImmediatePropagation?: boolean;
  canceled?: boolean;
  inPassiveListener?: boolean;
  composed?: boolean;
  initialized?: boolean;
  dispatch?: boolean;
}

const _type = Symbol("[[type]]");
const _target = Symbol("[[Target]]");
const _relatedTarget = Symbol("[[RelatedTarget]]");
const _currentTarget = Symbol("[[CurrentTarget]]");
const _touchTargetList = Symbol("[[TouchTargetList]]");
const _eventPhase = Symbol("[[EventPhase]]");
const _flags = Symbol("[[Flags]]");
const _bubbles = Symbol("[[Bubbles]]");
const _cancelable = Symbol("[[Cancelable]]");
const _isTrusted = Symbol("[[IsTrusted]]");
const _timeStamp = Symbol("[[TimeStamp]]");
const _path = Symbol("[[Path]]");

enum EventPhase {
  NONE = 0,
  CAPTURING_PHASE = 1,
  AT_TARGET = 2,
  BUBBLING_PHASE = 3,
}

interface EventPath {
  rootOfClosedTree: boolean;
  invocationTarget: EventTarget;
  shadowAdjustedTarget: EventTarget | null;
  relatedTarget: EventTarget | null;
  touchTargets: EventTarget[];
  slotInClosedTree: boolean;
}

class Event {
  [_type]: string;
  [_target]: EventTarget | null = null;
  [_relatedTarget]: EventTarget | null = null;
  [_currentTarget]: EventTarget | null = null;
  [_touchTargetList]: EventTarget[] = [];
  [_eventPhase]: EventPhase = EventPhase.NONE;
  [_flags]: EventFlags;
  [_bubbles]: boolean;
  [_cancelable]: boolean;
  [_isTrusted]: boolean = false;
  [_timeStamp]: number;
  [_path]: EventPath[] = [];

  constructor(type: string, eventInitDict?: EventInit) {
    this[_type] = type;
    this[_bubbles] = eventInitDict?.bubbles ?? false;
    this[_cancelable] = eventInitDict?.cancelable ?? false;
    this[_timeStamp] = Date.now();
    this[_flags] = {
      composed: eventInitDict?.composed ?? false,
      initialized: true,
    };
  }

  stopPropagation() {
    this[_flags].stopPropagation = true;
  }

  stopImmediatePropagation() {
    this[_flags].stopPropagation = true;
    this[_flags].stopImmediatePropagation = true;
  }

  preventDefault() {
    if (this[_cancelable] && !this[_flags].inPassiveListener) {
      this[_flags].canceled = true;
    }
  }

  composedPath(): EventTarget[] {
    // 1. Let composedPath be an empty list.
    const composedPath: EventTarget[] = [];
    // 2. Let path be this’s path.
    const path = this[_path];
    // 3. If path is empty, then return composedPath.
    if (path.length === 0) {
      return composedPath;
    }
    // 4. Let currentTarget be this’s currentTarget attribute value.
    const currentTarget = this[_currentTarget];
    assert(currentTarget !== null);
    // 5. Append currentTarget to composedPath.
    composedPath.push(currentTarget!);

    // 6. Let currentTargetIndex be 0.
    let currentTargetIndex = 0;
    // 7. Let currentTargetHiddenSubtreeLevel be 0.
    let currentTargetHiddenSubtreeLevel = 0;
    // 8. Let index be path’s size − 1.
    let index = path.length - 1;

    // Traverse the path backwards to find the current target index
    // 9. While index is greater than or equal to 0:
    while (index >= 0) {
      // 9.1 If path[index]'s root-of-closed-tree is true, then increase currentTargetHiddenSubtreeLevel by 1.
      if (path[index].rootOfClosedTree) {
        currentTargetHiddenSubtreeLevel += 1;
      }
      // 9.2 If path[index]'s invocation target is currentTarget, then set currentTargetIndex to index and break.
      if (path[index].invocationTarget === currentTarget) {
        currentTargetIndex = index;
        break;
      }
      // 9.3 If path[index]'s slot-in-closed-tree is true, then decrease currentTargetHiddenSubtreeLevel by 1.
      if (path[index].slotInClosedTree) {
        currentTargetHiddenSubtreeLevel -= 1;
      }
      // 9.4 Decrease index by 1.
      index -= 1;
    }

    // 10. Let currentHiddenLevel and maxHiddenLevel be currentTargetHiddenSubtreeLevel.
    let currentHiddenLevel = currentTargetHiddenSubtreeLevel;
    let maxHiddenLevel = currentTargetHiddenSubtreeLevel;

    // 11. Set index to currentTargetIndex − 1.
    index = currentTargetIndex - 1;
    // Traverse the path backwards from currentTargetIndex to prepend elements to composedPath
    // 12. While index is greater than or equal to 0:
    while (index >= 0) {
      // 12.1 If path[index]'s root-of-closed-tree is true, then increase currentHiddenLevel by 1.
      if (path[index].rootOfClosedTree) {
        currentHiddenLevel += 1;
      }
      // 12.2 If currentHiddenLevel is less than or equal to maxHiddenLevel,
      // then prepend path[index]'s invocation target to composedPath.
      if (currentHiddenLevel <= maxHiddenLevel) {
        composedPath.unshift(path[index].invocationTarget);
      }
      // 12.3 If path[index]'s slot-in-closed-tree is true, then:
      if (path[index].slotInClosedTree) {
        // 12.3.1 Decrease currentHiddenLevel by 1.
        currentHiddenLevel -= 1;
        // 12.3.2 If currentHiddenLevel is less than maxHiddenLevel, then set maxHiddenLevel to currentHiddenLevel.
        if (currentHiddenLevel < maxHiddenLevel) {
          maxHiddenLevel = currentHiddenLevel;
        }
      }
      // 12.4 Decrease index by 1.
      index -= 1;
    }
    // 13. Set currentHiddenLevel and maxHiddenLevel to currentTargetHiddenSubtreeLevel.
    currentHiddenLevel = currentTargetHiddenSubtreeLevel;
    maxHiddenLevel = currentTargetHiddenSubtreeLevel;
    // 14. Set index to currentTargetIndex + 1.
    index = currentTargetIndex + 1;
    // Traverse the path forwards from currentTargetIndex to append elements to composedPath
    // 15. While index is less than path’s size:
    while (index < path.length) {
      // 15.1 If path[index]'s slot-in-closed-tree is true, then increase currentHiddenLevel by 1.
      if (path[index].slotInClosedTree) {
        currentHiddenLevel += 1;
      }
      // 15.2 If currentHiddenLevel is less than or equal to maxHiddenLevel,
      // then append path[index]'s invocation target to composedPath.
      if (currentHiddenLevel <= maxHiddenLevel) {
        composedPath.push(path[index].invocationTarget);
      }
      // 15.3 If path[index]'s root-of-closed-tree is true, then:
      if (path[index].rootOfClosedTree) {
        // 15.3.1 Decrease currentHiddenLevel by 1.
        currentHiddenLevel -= 1;
        // 15.3.2 If currentHiddenLevel is less than maxHiddenLevel, then set maxHiddenLevel to currentHiddenLevel.
        if (currentHiddenLevel < maxHiddenLevel) {
          maxHiddenLevel = currentHiddenLevel;
        }
      }
      // 15.4 Increase index by 1.
      index += 1;
    }
    // 16. Return composedPath.
    return composedPath;
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

  get eventPhase(): EventPhase {
    return this[_eventPhase];
  }

  get cancelable() {
    return this[_cancelable];
  }

  get bubbles() {
    return this[_bubbles];
  }

  get defaultPrevented(): boolean {
    return !!this[_flags].canceled;
  }

  get composed(): boolean {
    return !!this[_flags].composed;
  }

  get isTrusted(): boolean {
    return this[_isTrusted];
  }

  get timeStamp(): number {
    return this[_timeStamp];
  }

  // Event phase constants
  static readonly NONE = EventPhase.NONE;
  static readonly CAPTURING_PHASE = EventPhase.CAPTURING_PHASE;
  static readonly AT_TARGET = EventPhase.AT_TARGET;
  static readonly BUBBLING_PHASE = EventPhase.BUBBLING_PHASE;
}

// | ---------------------------------------------------- |
// |  https://dom.spec.whatwg.org/#interface-eventtarget  |
// |                     EventTarget                      |
// | ---------------------------------------------------- |

interface EventListener {
  handleEvent(event: Event): void;
}

interface EventListenerOptions {
  capture?: boolean;
}

interface AddEventListenerOptions extends EventListenerOptions {
  passive?: boolean;
  once?: boolean;
  signal?: AbortSignal | null;
}

interface EventListenerRecord {
  type: string;
  callback: EventListener | null;
  capture: boolean;
  passive: boolean | null;
  once: boolean;
  signal: AbortSignal | null;
  removed?: boolean;
}

const flattenMoreOptions = (options?: AddEventListenerOptions | boolean) => {
  if (typeof options === "boolean") {
    return { capture: options, passive: false, once: false, signal: null };
  }

  return {
    capture: options?.capture ?? false,
    passive: options?.passive ?? false,
    once: options?.once ?? false,
    signal: options?.signal ?? null,
  };
};

// https://dom.spec.whatwg.org/#retarget
const retarget = <T = EventTarget>(relatedTarget: T, target: T) => {
  // Retargeting logic here, similar to the DOM retargeting algorithm
  return relatedTarget; // TODO: Implement retargeting logic
};

// https://dom.spec.whatwg.org/#concept-event-path-append
// To append to an event path, given an event, invocationTarget, shadowAdjustedTarget, relatedTarget,
// touchTargets, and a slot-in-closed-tree, run these steps:
// Let invocationTargetInShadowTree be false.
// If invocationTarget is a node and its root is a shadow root, then set invocationTargetInShadowTree to true.
// Let root-of-closed-tree be false.
// If invocationTarget is a shadow root whose mode is "closed", then set root-of-closed-tree to true.
// Append a new struct to event’s path whose invocation target is invocationTarget,
// invocation-target-in-shadow-tree is invocationTargetInShadowTree, shadow-adjusted target
// is shadowAdjustedTarget, relatedTarget is relatedTarget, touch target list is touchTargets,
// root-of-closed-tree is root-of-closed-tree, and slot-in-closed-tree is slot-in-closed-tree.
const appendEventPath = (
  event: Event,
  invocationTarget: EventTarget,
  shadowAdjustedTarget: EventTarget,
  relatedTarget: EventTarget | null,
  touchTargets: EventTarget[],
  slotInClosedTree: boolean,
) => {
  // 1. Let invocationTargetInShadowTree be false.
  let invocationTargetInShadowTree = false;
  // 2. If invocationTarget is a node and its root is a shadow root, then set invocationTargetInShadowTree to true.
  // TODO: Implement this check
  // 3. Let root-of-closed-tree be false.
  let rootOfClosedTree = false;
  // 4. If invocationTarget is a shadow root whose mode is "closed", then set root-of-closed-tree to true.
  // TODO: Implement this check
  // 5. Append a new struct to event’s path whose invocation target is invocationTarget, invocation-target-in-shadow-tree is invocationTargetInShadowTree, shadow-adjusted target is shadowAdjustedTarget, relatedTarget is relatedTarget, touch target list is touchTargets, root-of-closed-tree is root-of-closed-tree, and slot-in-closed-tree is slot-in-closed-tree.
  event[_path].push({
    invocationTarget,
    shadowAdjustedTarget,
    relatedTarget,
    touchTargets,
    rootOfClosedTree,
    slotInClosedTree,
  });
};

// https://dom.spec.whatwg.org/#dispatching-events
const dispatch = (event: Event, target: EventTarget) => {
  // 1. Set event’s dispatch flag.
  event[_flags].dispatch = true;
  // 2. Let targetOverride be target
  let targetOverride = target;
  // 3. Let activationTarget be null.
  let activationTarget = null;
  // 4. Let relatedTarget be the result of retargeting event’s relatedTarget against target.
  let relatedTarget = retarget(event[_relatedTarget], target);
  // 5. If target is not relatedTarget or target is event’s relatedTarget, then:
  if (target !== relatedTarget || target === event[_relatedTarget]) {
    // 5.1. Let touchTargets be a new list.
    const touchTargets: EventTarget[] = [];
    // 5.2. For each touchTarget of event’s touch target list, append the result of retargeting touchTarget against target to touchTargets.
    for (const touchTarget of event[_touchTargetList]) {
      touchTargets.push(retarget(touchTarget, target));
    }
    // 5.3. Append to an event path with event, target, targetOverride, relatedTarget, touchTargets, and false.
    appendEventPath(
      event,
      target,
      targetOverride,
      relatedTarget,
      touchTargets,
      false,
    );
    // 5.4 Let isActivationEvent be true, if event is a MouseEvent object and event’s type attribute is "click"; otherwise false.
    const isActivationEvent = false; // TODO: Implement this check
    // 5.5. If isActivationEvent is true and target has activation behavior, then set activationTarget to target.
    // TODO: Implement this check
    // 5.6. Let slottable be target, if target is a slottable and is assigned, and null otherwise.
    const slottable = null; // TODO: Implement this check
    // 5.7. Let slot-in-closed-tree be false.
    let slotInClosedTree = false;
    // 5.8. Let parent be the result of invoking target’s get the parent with event.
  }
};

const _eventListeners = Symbol("[eventListeners]");

class EventTarget {
  [_eventListeners]: EventListenerRecord[] = [];

  constructor() { }

  addEventListener(
    type: string,
    callback: EventListener | null,
    options?: AddEventListenerOptions | boolean,
  ): void {
    const { capture, passive, once, signal } = flattenMoreOptions(options);

    const listener: EventListenerRecord = {
      type,
      callback,
      capture,
      passive,
      once,
      signal,
      removed: false,
    };

    // 1. If listener’s signal is not null and is aborted, then return.
    if (listener.signal && listener.signal.aborted) {
      return;
    }
    // 2. If listener’s callback is null, then return
    if (listener.callback === null) {
      return;
    }

    const self = this;
    // 3. If listener’s passive is null, then set it to the default passive value given listener’s type and eventTarget.
    // 4. If eventTarget’s event listener list does not contain an event listener whose type is listener’s type, callback is listener’s callback, and capture is listener’s capture, then append listener to eventTarget’s event listener list.
    // check step 4
    const found = self[_eventListeners].find(
      (l) =>
        l.type === listener.type &&
        l.callback === listener.callback &&
        l.capture === listener.capture,
    );
    if (found) return;

    self[_eventListeners].push(listener);
    // 5. If listener’s signal is not null, then add the following abort steps to it:
    // 5.1 Remove an event listener with eventTarget and listener.
    if (listener.signal !== null) {
      listener.signal.addEventListener("abort", () => {
        self.removeEventListener(
          listener.type,
          listener.callback,
          listener.capture,
        );
      });
    }
  }

  removeEventListener(
    type: string,
    callback: EventListener | null,
    options?: EventListenerOptions | boolean,
  ): void {
    const capture = typeof options === "boolean" ? options : options?.capture;
    const listener = this[_eventListeners].find(
      (l) =>
        l.type === type && l.callback === callback && l.capture === capture,
    );
    if (!listener) return;

    listener.removed = true;
    // remove from list
    this[_eventListeners] = this[_eventListeners].filter((l) => l !== listener);
  }

  dispatchEvent(event: Event): boolean {
    // 1. If event’s dispatch flag is set, or if its initialized flag is not set, then throw an "InvalidStateError" DOMException.
    if (event[_flags].dispatch || !event[_flags].initialized) {
      throw new DOMException("Invalid state error", "InvalidStateError");
    }
    // 2. Initialize event’s isTrusted attribute to false.
    event[_isTrusted] = false;
    // 3. Return the result of dispatching event to this.
    return false;
  }
}

export { Event, EventTarget };
