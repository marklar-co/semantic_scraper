let linksList = [];
let visitedLinksList = [];
let contentDictionary = {};
let config = {};

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
    } else if (request.action === 'ping') {
        runPingTest();
    } else if (request.action === 'getTopics') {
        runGetTextTopicsTest();
    } else if (request.action === 'getTopicsError') {
        runGetTextTopicsErrorTest();
    } else {
        console.error('Unknown local message:', request);
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
    console.log('Connecting to native app');
    port = chrome.runtime.connectNative('ai.chamomile.semantic_collector');
    port.onMessage.addListener(onNativeMessage);
    port.onDisconnect.addListener(onDisconnected);
}

function onNativeMessage(message) {
    if (message.type === "Pong") {
        console.log("Recieved Pong:", message.req_id);
    } else if (message.type === "UpdateConfig") {
        config[message.key] = message.value;
        console.log("Recieved Config:", message.key, "=", message.value, config);
    } else if (message.type === "ReturnTextTopics") {
        console.log("Recieved Text topics:", message.req_id, message.topics);
    } else if (message.type === "Error") {
        console.error("Recieved ERROR:", message.req_id, message.error);
    } else {
        console.error("Unknown native message:", message)
    }
}

function onDisconnected() {
    if (chrome.runtime.lastError) {
        console.error('Error connecting to native app:', chrome.runtime.lastError.message);
    } else {
        console.error('Disconnected from native app');
    }
    port = null;
}

function sendReadingTextClassifyRequest(readingText, url) {
    if (!port) {
        console.error('Native app is not connected');
        return;
    }

    postMessageNative({
        type: 'GetTextTopics',
        url: 'http://example.com',
        req_id: req_id++,
        text: 'lorem ipsum dolor sit amet'
    });
}

function postMessageNative(message) {
    if (!port) {
        console.error('Native app is not connected');
        return;
    }

    console.log('Sending message to native app:', message);
    port.postMessage(message);
    console.log('Message sent to native app');
}

function runPingTest() {
    postMessageNative({
        type: 'Ping',
        req_id: req_id++
    });
}

function runGetTextTopicsTest() {
    const requests = [
        {
            url: 'https://lorem.com/index.html',
            text: 'Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.',
        },
        {
            url: 'https://emptytext.com',
            text: '',
        },
        {
            url: 'https://longtext.com',
            text: 'Lorem ipsum '.repeat(100) + 'dolor sit amet.',
        },
        {
            url: 'https://vitae.com',
            text: ' vitae  interdum, posuere ullamcorper ac ac sit amet justo. curabitur Posuere  et ',
        },
        {
            url: 'https://foreign.lang.com',
            text: '这是.一个中文测.试文本',
        },
        {
            url: 'https://whitespace.com',
            text: '  \n \n ',
        },
    ];

    for (const { url, text } of requests) {
        postMessageNative({
            type: 'GetTextTopics',
            req_id: req_id++,
            url,
            text,
        });
    }
}

function runGetTextTopicsErrorTest() {
    postMessageNative({
        type: 'GetTextTopics',
        req_id: req_id++,
        url: '',
        text: 'Lorem ipsum',
    });
}

// Example usage
chrome.runtime.onInstalled.addListener(() => {
    connectNative();
    setTimeout(() => {
        runGetTextTopicsTest();
        sendReadingTextClassifyRequest('Example reading text', 'http://example.com');
    }, 1000); // Delay to ensure connection is established
});

chrome.runtime.onStartup.addListener(() => {
    connectNative();
    setTimeout(() => {
        sendReadingTextClassifyRequest('Example reading text', 'http://example.com');
    }, 1000); // Delay to ensure connection is established
});
