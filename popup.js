document.getElementById('crawlButton').addEventListener('click', function () {
    startCrawl(true);
});

document.getElementById('collectButton').addEventListener('click', function () {
    startCrawl(false);
});

document.getElementById('clearButton').addEventListener('click', function () {
    chrome.runtime.sendMessage({
        action: 'clearData'
    }, response => {
        console.log('Background script response:', response);
    });
});

document.getElementById('downloadButton').addEventListener('click', function () {
    chrome.runtime.sendMessage({
        action: 'getData'
    }, response => {
        if (response && response.contentDictionary) {
            const jsonData = JSON.stringify(response.contentDictionary, null, 2); // Prettify JSON with 2 spaces indentation
            const blob = new Blob([jsonData], { type: 'application/json' });
            const url = URL.createObjectURL(blob);

            chrome.downloads.download({
                url: url,
                filename: 'collection.json',
                saveAs: true
            }, () => {
                URL.revokeObjectURL(url);
                console.log('Content dictionary downloaded as collection.json');
            });
        } else {
            console.error('Failed to get content dictionary from background script');
        }
    });
});

function startCrawl(shouldCrawl) {
    const inclusionPattern = document.getElementById('inclusionPattern').value;
    const excludedLinks = document.getElementById('excludedLinks').value.split(',').map(link => link.trim());

    chrome.tabs.query({ active: true, currentWindow: true }, function (tabs) {
        collectAndNavigate(tabs[0].id, inclusionPattern, excludedLinks, shouldCrawl);
    });
}

function collectAndNavigate(tabId, inclusionPattern, excludedLinks, shouldCrawl) {
    chrome.scripting.executeScript(
        {
            target: { tabId: tabId },
            function: collectPageContent,
            args: [inclusionPattern, excludedLinks]
        },
        (results) => {
            if (results && results[0]) {
                const { url, pageContent, links } = results[0].result;
                console.log("Url:", url);
                console.debug("Page Content:", pageContent);
                console.log("Links:", links);

                saveContentAndNavigate(tabId, url, pageContent, links, inclusionPattern, excludedLinks, shouldCrawl);
            }
        }
    );
}

function saveContentAndNavigate(tabId, url, pageContent, links, inclusionPattern, excludedLinks, shouldCrawl) {
    chrome.runtime.sendMessage({
        action: 'saveContent',
        data: {
            url: url,
            content: pageContent,
            links: links
        }
    }, response => {
        console.log('Background script response:', response);

        if (shouldCrawl) {
            chrome.runtime.sendMessage({ action: 'getNextLink' }, response => {
                const nextLink = response.nextLink;
                if (nextLink) {
                    chrome.tabs.update(tabId, { url: nextLink }, () => {
                        // Wait for the tab to load before collecting content again
                        chrome.tabs.onUpdated.addListener(function listener(tabIdUpdated, changeInfo) {
                            if (tabId === tabIdUpdated && changeInfo.status === 'complete') {
                                chrome.tabs.onUpdated.removeListener(listener);
                                const loadDelay = document.getElementById('loadDelay').value;
                                handleLoadDelay(tabId, inclusionPattern, excludedLinks, shouldCrawl, loadDelay);
                            }
                        });
                    });
                } else {
                    console.log('Crawl complete');
                }
            });
        }
    });
}

async function handleLoadDelay(tabId, inclusionPattern, excludedLinks, shouldCrawl, loadDelay) {
    const delay_s = loadDelay * 1000;
    await new Promise(resolve => setTimeout(resolve, delay_s));
    console.log(`${loadDelay} sec delay complete`);
    collectAndNavigate(tabId, inclusionPattern, excludedLinks, shouldCrawl);
}

async function collectPageContent(inclusionPattern, excludedLinks) {
    try {
        await new Promise((resolve) => {
            if (document.readyState === 'complete') {
                resolve();
            } else {
                window.onload = resolve;
            }
        });


        const pageContent = document.body.innerHTML;
        const currentHost = window.location.host;
        const currentUrl = window.location.href;
        console.log('Collecting contents of URL:', currentUrl);
        const links = Array.from(document.querySelectorAll('a'))
            .map(link => link.href)
            .filter(link => {
                try {
                    const url = new URL(link);
                    const matchesPattern = new RegExp(inclusionPattern).test(link);
                    const isExcluded = excludedLinks.includes(link);
                    const isAnchorLink = /#.*$/.test(link);
                    const isSameHost = url.host === currentHost;
                    return matchesPattern && !isExcluded && !isAnchorLink && isSameHost;
                } catch (e) {
                    console.error('Invalid URL:', link, e);
                    return false;
                }
            });

        // Filter out visited links
        const filteredLinks = [];
        for (const link of links) {
            const isVisited = await new Promise((resolve) => {
                chrome.runtime.sendMessage({ action: 'isVisitedLink', url: link }, response => {
                    resolve(response.isVisited);
                });
            });
            if (!isVisited) {
                filteredLinks.push(link);
            }
        }

        return { url: currentUrl, pageContent, links: filteredLinks };
    } catch (error) {
        console.error('Error collecting page content:', error);
        return { url: '', pageContent: '', links: [] };
    }
}
