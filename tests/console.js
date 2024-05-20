
const Data = {
    name: "Kevin",
    age: 25,
};

console.log("Hello, world!", Data);
console.log("Hello, world!", Data, Data, "Kevin", 44, null, undefined, true, false);
console.error("This is an error message.");
console.warn("This is a warning message.");
console.info("This is an info message.");
console.log("Hello, %s. You've called me %d times. %o Data", "Bob", 1, Data);

// for (let i = 0; i < 1000000; i++) {
//     console.log("Hello, %s. You've called me %d times.", "Bob", i + 1);
// }

export const test = "Kevin Test!";

