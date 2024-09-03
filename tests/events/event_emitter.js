import {
  deepStrictEqual,
  strictEqual,
  throws,
  doesNotThrow,
  ok,
} from "@kedo/assert";
import { EventEmitter } from "@kedo/events";

function test(description, fn) {
  try {
    fn();
    console.log(`✔️ ${description}`);
  } catch (error) {
    console.error(`❌  ${description}`);
    console.error(error.toString());
  }
}

async function asyncTest(description, fn) {
  try {
    await fn();
    console.log(`✔️ ${description}`);
  } catch (error) {
    console.error(`❌  ${description}`);
    console.error(error.toString());
  }
}

test("should add and call a listener", () => {
  const emitter = new EventEmitter();
  let called = false;
  const listener = (data) => {
    called = true;
    strictEqual(data, "test");
  };
  emitter.on("event", listener);
  strictEqual(emitter.listenerCount("event"), 1);
  emitter.emit("event", "test");
  ok(called, "Listener was not called");
});

test("should add and call a once listener only once", () => {
  const emitter = new EventEmitter();
  let callCount = 0;
  const listener = (data) => {
    callCount++;
    strictEqual(data, "test");
  };
  emitter.once("event", listener);
  strictEqual(emitter.listenerCount("event"), 1);
  emitter.emit("event", "test");
  emitter.emit("event", "test");
  strictEqual(callCount, 1);
  strictEqual(emitter.listenerCount("event"), 0);
});

test("should remove a listener", () => {
  const emitter = new EventEmitter();
  const listener = (data) => {
    throw new Error("Listener should not be called");
  };
  emitter.on("event", listener);
  emitter.off("event", listener);
  strictEqual(emitter.listenerCount("event"), 0);
  emitter.emit("event", "test");
});

test("should remove all listeners for an event", () => {
  const emitter = new EventEmitter();
  const listener1 = () => {};
  const listener2 = () => {};
  emitter.on("event", listener1);
  emitter.on("event", listener2);
  strictEqual(emitter.listenerCount("event"), 2);
  emitter.removeAllListeners("event");
  strictEqual(emitter.listenerCount("event"), 0);
});

test("should remove all listeners for all events", () => {
  const emitter = new EventEmitter();
  const listener1 = () => {};
  const listener2 = () => {};
  emitter.on("event1", listener1);
  emitter.on("event2", listener2);
  strictEqual(emitter.listenerCount("event1"), 1);
  strictEqual(emitter.listenerCount("event2"), 1);
  emitter.removeAllListeners();
  strictEqual(emitter.listenerCount("event1"), 0);
  strictEqual(emitter.listenerCount("event2"), 0);
});

test("should handle errors in listeners", () => {
  const emitter = new EventEmitter();
  const errorListener = (err) => {
    strictEqual(err.message, "Test error");
  };
  emitter.on(EventEmitter.errorEvent, errorListener);

  const listener = () => {
    throw new Error("Test error");
  };
  emitter.on("event", listener);
  emitter.emit("event");
});

await asyncTest("should handle errors in async listeners", async () => {
  const emitter = new EventEmitter();
  let errorCaught = false;
  const errorListener = (err) => {
    strictEqual(err.message, "Async error");
    errorCaught = true;
  };
  emitter.on(EventEmitter.errorEvent, errorListener);

  const asyncListener = async () => {
    throw new Error("Async error");
  };
  emitter.on("event", asyncListener);
  emitter.emit("event");
  await new Promise((resolve) => setTimeout(resolve, 10)); // Give the promise time to reject
  ok(errorCaught, "Error was not caught");
});

test("should respect max listeners", () => {
  const emitter = new EventEmitter();
  emitter.setMaxListeners(1);
  emitter.on("event", () => {});
  doesNotThrow(() => emitter.on("event", () => {}));
  strictEqual(emitter.listenerCount("event"), 2);
});

test("should return correct event names", () => {
  const emitter = new EventEmitter();
  emitter.on("event1", () => {});
  emitter.on("event2", () => {});
  deepStrictEqual(emitter.eventNames(), ["event1", "event2"]);
});

await asyncTest("should handle promise rejections in listeners", async () => {
  const emitter = new EventEmitter();
  let errorCaught = false;
  const errorListener = (err) => {
    strictEqual(err.message, "Promise rejection");
    errorCaught = true;
  };
  emitter.on(EventEmitter.errorEvent, errorListener);

  emitter.on("event", async () => {
    return Promise.reject(new Error("Promise rejection"));
  });

  emitter.emit("event");
  await new Promise((resolve) => setTimeout(resolve, 10)); // Give the promise time to reject
  ok(errorCaught, "Error was not caught");
});

await asyncTest("should handle listeners returning promises", async () => {
  const emitter = new EventEmitter();
  let promiseResolved = false;
  emitter.on("event", async () => {
    return new Promise((resolve) => {
      setTimeout(() => {
        promiseResolved = true;
        resolve();
      }, 10);
    });
  });

  emitter.emit("event");
  await new Promise((resolve) => setTimeout(resolve, 20)); // Give the promise time to resolve
  ok(promiseResolved, "Promise was not resolved");
});

test("should handle error in error listener", () => {
  const emitter = new EventEmitter();
  let monitorCalled = false;
  emitter.on(EventEmitter.errorMonitor, (err) => {
    strictEqual(err.message, "Test error");
    monitorCalled = true;
  });

  const errorListener = () => {
    throw new Error("Error in error listener");
  };

  emitter.on(EventEmitter.errorEvent, errorListener);

  const listener = () => {
    throw new Error("Test error");
  };

  emitter.on("event", listener);
  throws(() => emitter.emit("event"));
  ok(monitorCalled, "Error monitor was not called");
});
