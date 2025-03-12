chrome.runtime.onMessage.addListener((request, _sender, sendResponse) => {
    if (request.action === 'ping') {
        runPingTest();
        sendResponse("pong");
    }
});

let port = null;
let req_id = 7;

function connectNative() {
    console.log("Connecting to native app");
    port = chrome.runtime.connectNative('ai.chamomile.nativeext');
    port.onMessage.addListener(onNativeMessage);
    port.onDisconnect.addListener(onDisconnected);
}

function onNativeMessage(message) {
    console.log('Received message from native app:', message);
    // Handle the message from the native app
}

function onDisconnected() {
    if (chrome.runtime.lastError) {
        console.error("Error connecting to native app:", chrome.runtime.lastError.message);
    } else {
        console.error("Disconnected from native app");
    }
    port = null;
}

function runPingTest() {
    for (let i = 0; i < 10; ++i) {
        sendPing();
    }
}

function sendPing() {
    if (!port) {
        console.error('Native app is not connected');
        return;
    }

    const message = {
        type: 'Ping',
        req_id: req_id++
    };

    console.log('Sending message to native app:', message);
    port.postMessage(message);
    console.log('Message sent to native app');
}

// Example usage
chrome.runtime.onInstalled.addListener(() => {
    connectNative();
    setTimeout(() => {
        runPingTest();
        //
    }, 1000); // Delay to ensure connection is established
});

chrome.runtime.onStartup.addListener(() => {
    connectNative();
    setTimeout(() => {
        //
    }, 1000); // Delay to ensure connection is established
});
