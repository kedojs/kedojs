const timeouts = [1000, 2000, 3000];

const context = await Kedo.readFile("tests/filesystem/data.txt");
console.log(context);

const context2 = await Kedo.readFile("tests/filesystem/data.txt");
console.log(context2);

const context3 = await Kedo.readFile("tests/filesystem/data.txt");
console.log(context3);

let id = setTimeout(() => {
    console.log("Timeout Global executed");
}, 8000);

timeouts.forEach((duration, index) => {
    setTimeout(() => {

        Kedo.readFile("tests/filesystem/data.txt").then((context) => {
            console.log(context3);
            setTimeout(() => {
                console.log("After Context Log: %d executed", index + 1);
            }, 1000);
        });

        console.log(`Timeout ${index + 1} executed`);
        setTimeout(() => {
            console.log("Timeout Inner %d executed", index + 1);
        }
            , 1000);

    }, duration);
}
);

Kedo.readFile("tests/filesystem/data.txt").then((context) => {
    console.log("context4");
    setTimeout(() => {
        console.log("OutSide : %d executed", 8);
        setTimeout(() => {
            console.log("Deep level 1 : %d executed", 8);
            setTimeout(() => {
                console.log("Deep level 2 : %d executed", 8);
                setTimeout(() => {
                    console.log("Deep level 3 : %d executed", 8);
                    setTimeout(() => {
                        console.log("Deep level 5 : %d executed", 8);
                    }, 1000);
                }, 1000);
                setTimeout(() => {
                    console.log("Deep level 4 : %d executed", 8);
                }, 1000);
            }, 1000);
        }, 1000);
    }, 1000);
});

Kedo.readFile("tests/filesystem/data.txt").then((context) => {
    console.log("context5");
    setTimeout(() => {
        console.log("OutSide: %d executed", 9);
        setTimeout(() => {
            console.log("Deep level 22 : %d executed", 8);
            setTimeout(() => {
                console.log("Deep level 23 : %d executed", 8);
                setTimeout(() => {
                    console.log("Deep level 25 : %d executed", 8);
                }, 1000);
            }, 1000);
            setTimeout(() => {
                console.log("Deep level 24 : %d executed", 8);
            }, 1000);
        }, 1000);
    }, 1000);
});

console.log("Timeout 1 ID:", id);