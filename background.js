const config = {};

let port = null;
let req_id = 7;

chrome.runtime.onMessage.addListener((request, _sender, sendResponse) => {
    console.log(request.from);
    if (request.action === "ping") {
        runPingTest();
        sendResponse("pong");
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
        console.log("Pong:", message.req_id);
    } else if (message.type === "UpdateConfig") {
        config[message.key] = message.value;
        console.log("Config:", message.key, "=", message.value, config);
    } else if (message.type === "ReturnTextTopics") {
        console.log("Text topics:", message.req_id, message.topics);
    } else {
        console.error("Unknown message:", message)
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
    postMessageNative({
        type: "GetTextTopics",
        req_id: req_id++,
        url: "https://example.com",
        text: "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.",
    });
}

function runPingTest() {
    for (let i = 0; i < 10; ++i) {
        postMessageNative({
            type: "Ping",
            req_id: req_id++
        });
    }
}

function postMessageNative(message) {
    if (!port) {
        console.error("Native app is not connected");
        return;
    }
    port.postMessage(message);
}
