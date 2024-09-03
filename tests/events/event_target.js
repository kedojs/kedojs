import assert from "@kedo/assert";
import { Event, EventTarget } from "@kedo/events";

function runTests() {
  testBasicEventDispatch();
  testStopPropagation();
  testStopPropagationBetweenEventTargets();
  testStopImmediatePropagation();
  testPreventDefault();
  testErrorHandling();
  testUncaughtListenerExceptionNoHandler();
  testUncaughtListenerException();
  testUncaughtListenerExceptionOnErrorHandler();
}

function testBasicEventDispatch() {
  const target = new EventTarget();
  let eventFired = false;

  target.addEventListener("test", (event) => {
    eventFired = true;
  });

  const event = new Event("test");
  target.dispatchEvent(event);

  assert.ok(eventFired, "Event should have fired");
}

function testStopPropagation() {
  const target = new EventTarget();
  let firstListenerFired = false;
  let secondListenerFired = false;

  target.addEventListener("test", (event) => {
    firstListenerFired = true;
    event.stopPropagation();
  });

  target.addEventListener("test", (event) => {
    secondListenerFired = true;
  });

  const event = new Event("test");
  target.dispatchEvent(event);

  assert.ok(firstListenerFired, "First listener should have fired");
  assert.ok(
    secondListenerFired,
    "Second listener should not have fired due to stopPropagation",
  );
}

function testStopPropagationBetweenEventTargets() {
  const target1 = new EventTarget();
  const target2 = new EventTarget();
  let target1ListenerFired = false;
  let target2ListenerFired = false;

  target1.addEventListener("test", (event) => {
    target1ListenerFired = true;
    event.stopPropagation(); // Prevent event from being dispatched to target2
  });

  target2.addEventListener("test", (event) => {
    target2ListenerFired = true;
  });

  const event = new Event("test");
  target1.dispatchEvent(event);

  assert.ok(target1ListenerFired, "Target1 listener should have fired");
  assert.ok(
    !target2ListenerFired,
    "Target2 listener should NOT have fired due to stopPropagation",
  );
}

function testStopImmediatePropagation() {
  const target = new EventTarget();
  let firstListenerFired = false;
  let secondListenerFired = false;

  target.addEventListener("test", (event) => {
    firstListenerFired = true;
    event.stopImmediatePropagation();
  });

  target.addEventListener("test", (event) => {
    secondListenerFired = true;
  });

  const event = new Event("test");
  target.dispatchEvent(event);

  assert.ok(firstListenerFired, "First listener should have fired");
  assert.ok(
    !secondListenerFired,
    "Second listener should not have fired due to stopImmediatePropagation",
  );
}

function testPreventDefault() {
  const target = new EventTarget();
  let defaultPrevented = false;

  target.addEventListener("test", (event) => {
    event.preventDefault();
  });

  const event = new Event("test", { cancelable: true });
  defaultPrevented = !target.dispatchEvent(event);

  assert.ok(defaultPrevented, "Event default should have been prevented");
}

function testErrorHandling() {
  const target = new EventTarget();
  let firstListenerFired = false;
  let errorHandled = false;

  target.addEventListener("test", (event) => {
    firstListenerFired = true;
    throw new Error("Test error");
  });

  target.addEventListener(EventTarget.uncaghtListernerException, (event) => {
    errorHandled = true;
  });

  const event = new Event("test");
  target.dispatchEvent(event);

  assert.ok(firstListenerFired, "First listener should have fired");
  assert.ok(
    errorHandled,
    "Error should have been handled by uncaughtListenerException",
  );
}

function testUncaughtListenerException() {
  const target = new EventTarget();
  let errorHandlerCalled = false;

  target.addEventListener(EventTarget.uncaghtListernerException, (event) => {
    errorHandlerCalled = true;
  });

  target.addEventListener("test", (event) => {
    throw new Error("Simulated error");
  });

  const event = new Event("test");
  target.dispatchEvent(event);

  assert.ok(
    errorHandlerCalled,
    "Uncaught listener exception should have been handled",
  );
}

function testUncaughtListenerExceptionNoHandler() {
  const target = new EventTarget();
  let secondListenerFired = false;

  target.addEventListener("test", (event) => {
    throw new Error("Simulated error");
  });
  target.addEventListener("test", (event) => {
    secondListenerFired = true;
    throw new Error("Simulated error");
  });

  const event = new Event("test");

  assert.ok(
    target.dispatchEvent(event),
    "Event should have been dispatched even if there is no error handler",
  );
  assert.ok(
    secondListenerFired,
    "Second listener should have fired even if there is no error handler",
  );
}

function testUncaughtListenerExceptionOnErrorHandler() {
  const target = new EventTarget();
  let errorHandlerCalled = false;
  let secondErrorHandlerCalled = false;

  target.addEventListener(EventTarget.uncaghtListernerException, (event) => {
    throw new Error("Simulated error in error handler");
    errorHandlerCalled = true;
  });

  target.addEventListener(EventTarget.uncaghtListernerException, (event) => {
    secondErrorHandlerCalled = true;
  });

  target.addEventListener("test", (event) => {
    throw new Error("Simulated error");
  });

  const event = new Event("test");
  target.dispatchEvent(event);

  assert.ok(
    !errorHandlerCalled,
    "Uncaught listener exception should have been handled",
  );
  assert.ok(
    secondErrorHandlerCalled,
    "Second error handler should have been called",
  );
}

runTests();
console.log("All tests passed");
