document.getElementById("pingButton").addEventListener("click", function() {
    chrome.runtime.sendMessage({ action: "ping" }, _response => { });
});

document.getElementById("getTopicsButton").addEventListener("click", function() {
    chrome.runtime.sendMessage({ action: "getTopics" }, _response => { });
});

