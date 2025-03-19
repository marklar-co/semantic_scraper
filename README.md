# NativeExt

'NativeExt' is a placeholder name.

## Installation

### Load the extension

Clone this repo, then:

1) Go to `chrome://extensions`
2) Enable `Developer mode`
3) `Load unpacked`
4) Open the service worker logs by clicking the Inspect view link
5) Open the extension popup in the Chrome toolbar to simulate requests

### Set up the native extension

1) Build the native extension binary using `cargo`, or [download the latest release](https://github.com/dxrcy/nativeext/releases/latest)
2) Open `nativeext.json`
3) Change the `"path"` value to the **absolute** path of the native extension binary
4) Change the `"allowed-origins"` value to the extension ID\* (including trailing slash)

\*Extension ID can be found on the `chrome://extensions` listing.

#### Windows

1) Open `regedit`
2) Go to `Computer\HKEY_LOCAL_MACHINE\SOFTWARE\Google\Chrome\NativeMessagingHosts`
3) Add key `ai.chamomile.nativeext` with default value of path to `nativeext.json`

#### Linux

1) Go to `$XDG_CONFIG_HOME/google-chrome/NativeMessagingHosts` (replace `google-chrome` with `chromium` to use with Chromium)
2) Copy or link the `nativeext.json` here, with the filename `ai.chamomile.nativeext.json`

