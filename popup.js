document.getElementById("pingButton").addEventListener("click", function() {
    sendPing();
});

function sendPing() {
    chrome.runtime.sendMessage({ action: "ping" }, response => {
        console.log("response:", response);
    });
}

