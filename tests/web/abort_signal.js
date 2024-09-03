import assert from "@kedo/assert";

function runAbortSignalTests() {
  testAbortSignalBasicFunctionality();
  testAbortSignalReason();
  testAbortSignalTimeout();
  testAbortSignalAny();
  testAbortControllerAbort();
  testAbortControllerSignal();
  testAbortSignalDependentBehavior();
  testAbortSignalDependentBehaviorAnyWithtimeout();
  testAbortSignalEventListener();
}

function testAbortSignalBasicFunctionality() {
  const signal = new AbortController().signal;

  assert.ok(!signal.aborted, "AbortSignal should not be aborted by default");
  assert.strictEqual(
    signal.reason,
    undefined,
    "Abort reason should be undefined by default",
  );
}

function testAbortSignalReason() {
  const signal = AbortSignal.abort("Test abort reason");

  assert.ok(signal.aborted, "AbortSignal should be aborted");
  assert.strictEqual(
    signal.reason,
    "Test abort reason",
    "Abort reason should match the provided reason",
  );
}

function testAbortSignalTimeout() {
  const signal = AbortSignal.timeout(10);

  assert.ok(!signal.aborted, "AbortSignal should not be aborted immediately");

  setTimeout(() => {
    assert.ok(signal.aborted, "AbortSignal should be aborted after timeout");
    assert.ok(
      signal.reason instanceof DOMException,
      "Abort reason should be a DOMException",
    );
    assert.strictEqual(
      signal.reason.name,
      "TimeoutError",
      'Abort reason name should be "TimeoutError"',
    );
  }, 15);
}

function testAbortSignalAny() {
  const signal1 = new AbortController().signal;
  const signal2 = AbortSignal.abort("Test abort reason");
  const signalAny = AbortSignal.any([signal1, signal2]);

  assert.ok(
    signalAny.aborted,
    "AbortSignal.any should be aborted if one of the signals is aborted",
  );
  assert.strictEqual(
    signalAny.reason,
    signal2.reason,
    "AbortSignal.any should have the same reason as the aborted signal",
  );
}

function testAbortControllerAbort() {
  const controller = new AbortController();
  const signal = controller.signal;

  controller.abort("Controller abort reason");

  assert.ok(
    signal.aborted,
    "AbortSignal should be aborted after controller.abort is called",
  );
  assert.strictEqual(
    signal.reason,
    "Controller abort reason",
    "Abort reason should match the reason provided to controller.abort",
  );
}

function testAbortControllerSignal() {
  const controller = new AbortController();
  const signal = controller.signal;

  assert.ok(
    signal instanceof AbortSignal,
    "Controller signal should be an instance of AbortSignal",
  );
}

function testAbortSignalDependentBehavior() {
  const controller1 = new AbortController();
  const controller2 = new AbortController();
  const signal = AbortSignal.any([controller1.signal, controller2.signal]);

  assert.ok(
    !signal.aborted,
    "Dependent AbortSignal should not be aborted initially",
  );

  controller1.abort("Abort by controller1");
  assert.ok(
    signal.aborted,
    "Dependent AbortSignal should be aborted when one of the source signals is aborted",
  );
  assert.strictEqual(
    signal.reason,
    "Abort by controller1",
    "Dependent AbortSignal reason should match the first aborted signal's reason",
  );
}

function testAbortSignalDependentBehaviorAnyWithtimeout() {
  const controller1 = new AbortController();
  const signal2 = AbortSignal.timeout(10);
  const signal = AbortSignal.any([controller1.signal, signal2]);

  assert.ok(
    !signal.aborted,
    "Dependent AbortSignal should not be aborted initially",
  );

  setTimeout(() => {
    assert.ok(
      signal.aborted,
      "Dependent AbortSignal should be aborted after timeout",
    );
    assert.ok(
      signal.reason instanceof DOMException,
      "Dependent AbortSignal reason should be a DOMException",
    );
    assert.strictEqual(
      signal.reason.name,
      "TimeoutError",
      'Dependent AbortSignal reason name should be "TimeoutError"',
    );
  }, 15);
}

function testAbortSignalEventListener() {
  const controller = new AbortController();
  const signal = controller.signal;
  let abortEventFired = false;

  signal.onabort = () => {
    abortEventFired = true;
  };

  controller.abort();

  assert.ok(
    abortEventFired,
    "Abort event should be fired when the signal is aborted",
  );
}

runAbortSignalTests();

console.log("All AbortSignal tests passed");
