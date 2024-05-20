// Set up an array of timeout durations
const timeouts = [1000, 2000, 3000];

// Function to execute after each timeout
// function timeoutCallback(index) {
//     console.log(`Timeout ${index + 1} executed`);
// }

// // Function to start the timeouts
// function startTimeouts() {
//     timeouts.forEach((duration, index) => {
//         setTimeout(() => {
//             timeoutCallback(index);
//         }, duration);
//     });
// }

// // Start the timeouts
// startTimeouts();

let id = setTimeout(() => {
    console.log("Timeout Global executed");
}, 2000);

timeouts.forEach((duration, index) => {
    setTimeout(() => {
        console.log(`Timeout ${index + 1} executed`);
        setTimeout(() => {
            console.log("Timeout Inner %d executed", index + 1);
        }
            , 1000);

    }, duration);
}
);

Kedo.readFile("tests/consolek.js").then((context) => {
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
                        setTimeout(() => {
                            console.log("Deep level 6 : %d executed", 8);
                            setTimeout(() => {
                                console.log("Deep level 7 : %d executed", 8);
                                setTimeout(() => {
                                    console.log("Deep level 8 : %d executed", 8);
                                    setTimeout(() => {
                                        console.log("Deep level 9 : %d executed", 8);
                                    }, 1000);
                                }, 1000);
                                setTimeout(() => {
                                    console.log("Deep level 4 : %d executed", 8);
                                }, 1000);
                            }, 1000);
                        }, 1000);
                    }, 1000);
                }, 1000);
                setTimeout(() => {
                    console.log("Deep level 4 : %d executed", 8);
                }, 1000);
            }, 1000);
        }, 1000);
    }, 1000);
}
).catch((error) => {
    console.log(error);
});

Kedo.readFile("tests/console.js").then((context) => {
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
                        setTimeout(() => {
                            console.log("Deep level 6 : %d executed", 8);
                            setTimeout(() => {
                                console.log("Deep level 7 : %d executed", 8);
                                setTimeout(() => {
                                    console.log("Deep level 8 : %d executed", 8);
                                    setTimeout(() => {
                                        console.log("Deep level 9 : %d executed", 8);
                                    }, 1000);
                                }, 1000);
                                setTimeout(() => {
                                    console.log("Deep level 4 : %d executed", 8);
                                }, 1000);
                            }, 1000);
                        }, 1000);
                    }, 1000);
                }, 1000);
                setTimeout(() => {
                    console.log("Deep level 4 : %d executed", 8);
                }, 1000);
            }, 1000);
        }, 1000);
    }, 1000);
}
);

Kedo.readFile("tests/console.js").then((context) => {
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
                        setTimeout(() => {
                            console.log("Deep level 6 : %d executed", 8);
                            setTimeout(() => {
                                console.log("Deep level 7 : %d executed", 8);
                                setTimeout(() => {
                                    console.log("Deep level 8 : %d executed", 8);
                                    setTimeout(() => {
                                        console.log("Deep level 9 : %d executed", 8);
                                        Kedo.readFile("tests/console.js").then((context) => {
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
                                                                setTimeout(() => {
                                                                    console.log("Deep level 6 : %d executed", 8);
                                                                    setTimeout(() => {
                                                                        console.log("Deep level 7 : %d executed", 8);
                                                                        setTimeout(() => {
                                                                            console.log("Deep level 8 : %d executed", 8);
                                                                            setTimeout(() => {
                                                                                console.log("Deep level KedoJS : %d executed", 8);
                                                                            }, 1000);
                                                                        }, 1000);
                                                                        setTimeout(() => {
                                                                            console.log("Deep level 4 : %d executed", 8);
                                                                        }, 1000);
                                                                    }, 1000);
                                                                }, 1000);
                                                            }, 1000);
                                                        }, 1000);
                                                        setTimeout(() => {
                                                            console.log("Deep level 4 : %d executed", 8);
                                                        }, 1000);
                                                    }, 1000);
                                                }, 1000);
                                            }, 1000);
                                        }
                                        );
                                    }, 1000);
                                }, 1000);
                                setTimeout(() => {
                                    console.log("Deep level 4 : %d executed", 8);
                                }, 1000);
                            }, 1000);
                        }, 1000);
                    }, 1000);
                }, 1000);
                setTimeout(() => {
                    console.log("Deep level 4 : %d executed", 8);
                }, 1000);
            }, 1000);
        }, 1000);
    }, 1000);
}
);

Kedo.readFile("tests/console.js").then((context) => {
    Kedo.readFile("tests/console.js").then((context) => {
        Kedo.readFile("tests/console.js").then((context) => {
        });
    }
    );
}
);
console.log("Timeout 1 ID:", id);

clearTimeout(id);