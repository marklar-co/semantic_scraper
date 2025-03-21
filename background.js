let linksList = [];
let visitedLinksList = [];
let contentDictionary = {};

chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
    if (request.action === 'saveContent') {
        saveContent(request.data);
        sendResponse({ status: 'success' });
    } else if (request.action === 'clearData') {
        clearData();
        sendResponse({ status: 'success' });
    } else if (request.action === 'getData') {
        sendResponse({ contentDictionary: contentDictionary });
    } else if (request.action === 'getNextLink') {
        const nextLink = getNextLink();
        console.log('Next link:', nextLink);
        sendResponse({ nextLink: nextLink });
    } else if (request.action === 'isVisitedLink') {
        const isVisited = visitedLinksList.includes(request.url);
        sendResponse({ isVisited: isVisited });
    }
});

function saveContent(data) {
    console.log('Saving content:', data);

    if (!data.url || !data.content || !data.links) {
        console.error('Invalid data:', data);
        return;
    }

    linksList = Array.from(new Set(linksList.concat(data.links)));
    console.log('Updated links list:', linksList);

    contentDictionary[data.url] = data.content;
    console.log('Updated content dictionary:', contentDictionary);

    visitedLinksList.push(data.url);
    console.log('Updated visited links list:', visitedLinksList);
}

function getNextLink() {
    let nextLink = null;
    while (linksList.length > 0) {
        nextLink = linksList.pop();
        if (!visitedLinksList.includes(nextLink)) {
            break;
        }
        nextLink = null;
    }
    return nextLink;
}

function clearData() {
    linksList = [];
    visitedLinksList = [];
    contentDictionary = {};
    console.log('Cleared links list, visited links list, and content dictionary');
}

let port = null;
let req_id = 7;

function connectNative() {
    console.log("Connecting to native app");
    port = chrome.runtime.connectNative('ai.chamomile.semantic_collector');
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

function sendReadingTextClassifyRequest(readingText, url) {
    if (!port) {
        console.error('Native app is not connected');
        return;
    }

    const message = {
        type: 'GetTextTopics',
        url: 'http://example.com',
        req_id: req_id++,
        text: 'lorem ipsum dolor sit amet'
    };

    console.log('Sending message to native app:', message);
    port.postMessage(message);
    console.log('Message sent to native app');
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
        for (let i = 0; i < 10; ++i) {
            sendPing();
        }
        sendReadingTextClassifyRequest('Example reading text', 'http://example.com');
    }, 1000); // Delay to ensure connection is established
});

chrome.runtime.onStartup.addListener(() => {
    connectNative();
    setTimeout(() => {
        sendReadingTextClassifyRequest('Example reading text', 'http://example.com');
    }, 1000); // Delay to ensure connection is established
});
