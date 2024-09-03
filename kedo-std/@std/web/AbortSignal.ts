// | -------------------------------------------- |
// |   https://dom.spec.whatwg.org/#abortsignal   |
// |                AbortSignal                   |
// | -------------------------------------------- |

import { IterableWeakSet } from "@kedo/ds";
import { queue_internal_timeout } from "@kedo/internal/utils";
import { EventTarget, Event } from "@kedo/events";
import { assert } from "../utils";
// import { DOMException } from "./utils";

const _abortReason = Symbol("[abortReason]");
const _abortAlgorithms = Symbol("[abortAlgorithms]");
const _dependent = Symbol("[dependent]");
const _sourceSignals = Symbol("[sourceSignals]");
const _dependentSignals = Symbol("[dependentSignals]");
const _illegalConstructor = Symbol("[illegalConstructor]");
const _signalAbort = Symbol("[signalAbort]");
const _addAlgorithm = Symbol("[addAlgorithm]");
const _removeAlgorithm = Symbol("[removeAlgorithm]");

interface AbortAlgorithm {
  (): void;
}

const fireAbortEvent = (signal: AbortSignal): boolean => {
  // 1. If eventConstructor is not given, then let eventConstructor be Event.
  // 2. Let event be the result of creating an event given eventConstructor, in the relevant realm of target.
  // 3. Initialize event’s type attribute to e.
  const event = new Event("abort");
  return signal.dispatchEvent(event);
};

class AbortSignal extends EventTarget {
  [_abortReason]: any = undefined;
  [_abortAlgorithms]: Set<AbortAlgorithm> = new Set();
  [_dependent]: boolean = false;
  [_sourceSignals]: IterableWeakSet<AbortSignal> = new IterableWeakSet();
  [_dependentSignals]: IterableWeakSet<AbortSignal> = new IterableWeakSet();

  constructor(key?: any) {
    if (key !== _illegalConstructor) {
      throw new TypeError("Illegal constructor.");
    }
    super();
  }

  get aborted(): boolean {
    return this[_abortReason] !== undefined;
  }

  get reason(): any {
    return this[_abortReason];
  }

  throwIfAborted(): void {
    if (this.aborted) {
      throw this.reason;
    }
  }

  static abort(reason: any): AbortSignal {
    // 1. Let signal be a new AbortSignal object.
    const signal = new AbortSignal(_illegalConstructor);
    // 2. Set signal’s abort reason to reason if it is given; otherwise to a new "AbortError"
    signal[_abortReason] =
      reason || new DOMException("Signal is aborted", "AbortError");
    // 3. Return signal.
    return signal;
  }

  static timeout(ms: number): AbortSignal {
    // 1. Let signal be a new AbortSignal object.
    const signal = new AbortSignal(_illegalConstructor);
    // 3. Run steps after a timeout given global, "AbortSignal-timeout", milliseconds, and the following step:
    queue_internal_timeout(() => {
      // 3.1 Queue a global task on the timer task source given global to signal abort given signal and a new "TimeoutError" DOMException.
      signal[_signalAbort](
        new DOMException("Signal timed out", "TimeoutError"),
      );
    }, ms);
    // 5. Return signal.
    return signal;
  }

  static any(signals: AbortSignal[]): AbortSignal {
    // 1. If signals is empty, then return a new AbortSignal object.
    if (signals.length === 0) {
      return new AbortSignal(_illegalConstructor);
    }

    // 1. Return createDependentAbortSignal(signals).
    return createDependentAbortSignal(signals);
  }

  set onabort(listener: EventListener) {
    this.addEventListener("abort", listener);
  }

  [_signalAbort](reason?: any): void {
    // 1. If signal is aborted, then return.
    if (this.aborted) return;
    // 2. Set signal’s abort reason to reason if it is given; otherwise to a new "AbortError"
    this[_abortReason] =
      reason || new DOMException("Signal is aborted", "AbortError");
    // 3. For each algorithm of signal’s abort algorithms: run algorithm.
    for (const algorithm of this[_abortAlgorithms]) {
      algorithm();
    }
    // 4. Empty signal’s abort algorithms.
    this[_abortAlgorithms] = new Set();
    // 5. Fire an event named abort at signal.
    fireAbortEvent(this);
    // 6. For each dependentSignal of signal’s dependent signals, signal abort on dependentSignal with signal’s abort reason.
    for (const dependentSignal of this[_dependentSignals]) {
      dependentSignal[_signalAbort](this.reason);
    }
  }

  // To add an algorithm algorithm to an AbortSignal object signal:
  // If signal is aborted, then return.
  // Append algorithm to signal’s abort algorithms.
  [_addAlgorithm](algorithm: AbortAlgorithm): void {
    if (this.aborted) return;
    this[_abortAlgorithms].add(algorithm);
  }

  // To remove an algorithm algorithm from an AbortSignal object signal:
  // Remove algorithm from signal’s abort algorithms.
  [_removeAlgorithm](algorithm: AbortAlgorithm): void {
    this[_abortAlgorithms].delete(algorithm);
  }
}

// https://dom.spec.whatwg.org/#create-a-dependent-abort-signal
const createDependentAbortSignal = (signals: AbortSignal[]): AbortSignal => {
  // 1. Let resultSignal be a new object implementing signalInterface using realm.
  const resultSignal = new AbortSignal(_illegalConstructor);
  // 2. For each signal of signals: if signal is aborted, then set resultSignal’s abort reason to signal’s abort reason and return resultSignal.
  for (const signal of signals) {
    if (signal.aborted) {
      resultSignal[_abortReason] = signal.reason;
      return resultSignal;
    }
  }

  // 3. Set resultSignal’s dependent to true.
  resultSignal[_dependent] = true;
  // 4. For each signal of signals:
  for (const signal of signals) {
    // 4.1. If signal’s dependent is false, then:
    if (!signal[_dependent]) {
      // 4.1.1. Append signal to resultSignal’s source signals.
      resultSignal[_sourceSignals].add(signal);
      // 4.1.2. Append resultSignal to signal’s dependent signals.
      signal[_dependentSignals].add(resultSignal);
    } else {
      // 4.2. Otherwise, for each sourceSignal of signal’s source signals:
      for (const sourceSignal of signal[_sourceSignals]) {
        // 4.2.1. Assert: sourceSignal is not aborted and not dependent.
        assert(!sourceSignal.aborted, "sourceSignal is aborted");
        assert(!sourceSignal[_dependent], "sourceSignal is dependent");
        // 4.2.1. Append sourceSignal to resultSignal’s source signals.
        resultSignal[_sourceSignals].add(sourceSignal);
        // 4.2.2. Append resultSignal to sourceSignal’s dependent signals.
        sourceSignal[_dependentSignals].add(resultSignal);
      }
    }
  }

  // 5. Return resultSignal.
  return resultSignal;
};

// https://dom.spec.whatwg.org/#interface-abortcontroller
// |----------------------------------------------------------|
// |                      AbortController                     |
// |----------------------------------------------------------|

const _signal = Symbol("signal");

class AbortController {
  [_signal]: AbortSignal;

  constructor() {
    // 1. Let signal be a new AbortSignal object.
    this[_signal] = new AbortSignal(_illegalConstructor);
  }

  get signal(): AbortSignal {
    // 1. Return signal.
    return this[_signal];
  }

  abort(reason?: any): void {
    // 1. If signal’s aborted flag is set, then return.
    if (this[_signal].aborted) return;
    // 2. Run signal’s signal abort with reason.
    this[_signal][_signalAbort](reason);
  }
}

export { AbortSignal, AbortController, createDependentAbortSignal };
