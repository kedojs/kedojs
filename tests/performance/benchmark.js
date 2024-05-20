(function performanceTest() {
    // console.time("PerformanceTest");

    // Loop performance
    for (let i = 0; i < 1000000; i++) { }

    // Mathematical computations
    let sum = 0;
    for (let i = 1; i <= 100000; i++) {
        sum += i;
    }

    // Array manipulations
    const arr = Array.from({ length: 100000 }, (_, index) => index);
    const reversedArray = arr.reverse();
    const filteredArray = arr.filter(x => x % 2 === 0);
    const mappedArray = arr.map(x => x * 2);

    // Object-oriented operations
    class Person {
        constructor(name, age) {
            this.name = name;
            this.age = age;
        }

        greet() {
            return `Hello, my name is ${this.name} and I am ${this.age} years old.`;
        }
    }

    const people = [];
    for (let i = 0; i < 10000; i++) {
        people.push(new Person(`Person ${i}`, i));
    }
    const greetings = people.map(person => person.greet());

    // console.timeEnd("PerformanceTest");
})();
