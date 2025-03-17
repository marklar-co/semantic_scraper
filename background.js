const config = {};

let port = null;
let req_id = 7;

chrome.runtime.onMessage.addListener((message, _sender, _sendResponse) => {
    if (message.action === "ping") {
        runPingTest();
    } else if (message.action === "getTopics") {
        runGetTextTopicsTest();
    } else {
        console.error("Unknown local message:", message);
    }
});

// Example usage
chrome.runtime.onInstalled.addListener(() => {
    connectNative();
    setTimeout(() => {
        runPingTest();
        runGetTextTopicsTest();
    }, 1000); // Delay to ensure connection is established
});

chrome.runtime.onStartup.addListener(() => {
    connectNative();
    setTimeout(() => {
        runGetTextTopicsTest();
    }, 1000); // Delay to ensure connection is established
});

function connectNative() {
    console.log("Connecting to native app");
    port = chrome.runtime.connectNative("ai.chamomile.nativeext");
    port.onMessage.addListener(onNativeMessage);
    port.onDisconnect.addListener(onDisconnected);
}

function onNativeMessage(message) {
    if (message.type === "Pong") {
        console.log("RECIEVED Pong:", message.req_id);
    } else if (message.type === "UpdateConfig") {
        config[message.key] = message.value;
        console.log("RECIEVED Config:", message.key, "=", message.value, config);
    } else if (message.type === "ReturnTextTopics") {
        console.log("RECIEVED Text topics:", message.req_id, message.topics);
    } else if (message.type === "Error") {
        console.error("RECIEVED ERROR:", message.req_id, message.error);
    } else {
        console.error("Unknown native message:", message)
    }
}

function onDisconnected() {
    if (chrome.runtime.lastError) {
        console.error("Error connecting to native app:", chrome.runtime.lastError.message);
    } else {
        console.error("Disconnected from native app");
    }
    port = null;
}

function runGetTextTopicsTest() {
    const requests = [
        {
            url: "https://lorem.com/index.html",
            text: "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.",
        },
        {
            url: "https://emptytext.com",
            text: "",
        },
        {
            url: "https://longtext.com",
            text: "Lorem ipsum ".repeat(100) + "dolor sit amet.",
        },
        {
            url: "https://vitae.com",
            text: " vitae  interdum, posuere ullamcorper ac ac sit amet justo. curabitur Posuere  et ",
        },
        {
            url: "https://foreign.lang.com",
            text: "这是.一个中文测.试文本",
        },
        {
            url: "https://whitespace.com",
            text: "  \n \n ",
        },
    ];

    for (const { url, text } of requests) {
        postMessageNative({
            type: "GetTextTopics",
            req_id: req_id++,
            url,
            text,
        });
    }
}

function runPingTest() {
    postMessageNative({
        type: "Ping",
        req_id: req_id++
    });
}

function postMessageNative(message) {
    if (!port) {
        console.error("Native app is not connected");
        return;
    }
    port.postMessage(message);
}
