# Semantic Collector

Semantic Collector (`SemCollect`) is a tool to collect data from websites, for use cases like
[Retrieval Augmented Generation](https://chamomile.ai/reliable-rag-with-data-preprocessing/).

## Basic usage

This hasn't been released on the web store yet, so for now this browser extension has to be loaded unpacked, in dev mode.

Clone this repo, then load unpacked, then:

1) Go to `chrome://extensions`
2) Enable `Developer mode`
3) `Load unpacked`
4) Open the service worker logs by clicking the Inspect view link (important to validate what's going on)
5) Save some content. Download. Click clear when you're done to reset internal state.

## Collect & Crawl

Clear Data, click Collect & Crawl, then sit back and watch. When things stop, click Download to save results.

**You must leave the popup in focus while crawl collection is happening.** You cannot use your browser while this operation
is running.

The collector is hardcoded to avoid links that cross over to new hostnames, *however*, in the case of redirects
we may end up on a new host. Therefore, it is *strongly recommended* to use the Link Inclusion Pattern regex,
e.g. `https://chamomile\.ai/.*`.

For extra protection, add specific URLs to exclude, e.g. those that are known to redirect.

## Limitations

This codebase is very new and has some limitations. Some limitations will be easy to solve, and some will be harder.

For limitations that require code changes, we're more than **happy to accept pull requests** for anything in
the easy section. For things in the hard section please get in touch first by filing an issue, to discuss the
approach.

### Easy to solve

* No crawl progress indicator (a practical approach would be to update links crawled/to crawl on the popup ui)
* Can't save link inclusion/exclusion specifications to repeat the same jobs
* Not tested at all on Edge (only tested on Chrome)
* Very limited testing on complex sites
* No metadata in the output file; we may, for instance, want to include collection date

### Harder to solve

* No scroll-to-complete-load functiionality
