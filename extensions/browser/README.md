# Rocky Browser Extension

Add content to your [Rocky](https://github.com/NVME-git/rocky) personal knowledge graph directly from your browser. Capture YouTube videos, articles, and other resources and link them to topics in your PKG.

> **Status**: Scaffolding & planned — not yet production-ready. The core popup, content script, and background worker are in place; the sync API endpoint in Rocky CLI is planned for a future release.

## Planned Features

| Feature | Description | Status |
|---------|-------------|--------|
| **Quick-add popup** | Click the Rocky icon to add the current page URL to one or more PKG topics | ✅ UI ready |
| **YouTube detection** | Auto-detects video title and channel on YouTube watch pages | ✅ Content script ready |
| **Right-click menu** | Right-click any link → "Add to Rocky PKG" | ✅ Context menu ready |
| **Topic search** | Search and select topics from your PKG in the popup | ✅ UI ready |
| **Local queue** | Resources are stored locally until synced | ✅ Storage ready |
| **Rocky sync API** | POST resources to a Rocky CLI endpoint for import | 🔜 Planned |
| **Auto-classification** | LLM-powered topic suggestion based on page content | 🔜 Planned |

## How It Works

1. Browse the web normally.
2. Find a resource (YouTube video, article, documentation) that explains a concept you're learning.
3. Click the Rocky extension icon or right-click → "Add to Rocky PKG".
4. The popup shows the detected URL and title.
5. Search and select one or more PKG topics to link the resource to.
6. Click **Add to Knowledge Graph** — the resource is queued locally.
7. When Rocky's sync API is available, queued resources are pushed to your PKG.

### YouTube Example

When watching a YouTube video about async/await in Rust:

1. The content script detects the video metadata automatically.
2. Open the Rocky popup — URL and title are pre-filled.
3. Search for "async/await" in the topic list and select it.
4. Click **Add** — the video is linked to your async/await topic.

## Prerequisites

- **Chrome/Chromium** (or any Manifest V3-compatible browser)
- A Rocky PKG with topics (run `rocky backup` to generate topic data)
- Rocky CLI installed for future sync functionality

## Installation

### Development (unpacked)

```bash
cd extensions/browser

# Install dependencies and build (when build tooling is added)
# npm install
# npm run build

# For now, load the extension directly:
# 1. Open chrome://extensions
# 2. Enable "Developer mode"
# 3. Click "Load unpacked"
# 4. Select the extensions/browser/ directory
```

### Importing Topics

Until the sync API is ready, you can manually export topics from Rocky:

```bash
# Export your topic names to a JSON array
rocky ls --json | jq '[.nodes[].name]' > topics.json
```

Then paste the array into the browser extension's storage via the DevTools console:

```js
chrome.storage.local.set({ rockyTopics: ["async/await", "JWT Auth", "Docker Compose"] });
```

## Configuration

The extension stores configuration in `chrome.storage.local`:

| Key | Type | Description |
|-----|------|-------------|
| `rockyTopics` | `string[]` | List of PKG topic names for the search UI |
| `pendingResources` | `object[]` | Queue of resources waiting to be synced |
| `pendingResource` | `object` | Most recent right-click capture |

## Architecture

```
extensions/browser/
├── manifest.json         # Chrome Manifest V3 extension config
├── icons/                # Extension icons (16/48/128px)
├── src/
│   ├── popup.html        # Popup UI (topic search, add button)
│   ├── popup.ts          # Popup logic (tab detection, topic selection)
│   ├── background.ts     # Service worker (context menu, message handling)
│   └── content.ts        # Content script (YouTube metadata extraction)
└── README.md             # This file
```

### Data Flow

```
Browser tab  ──→  Content Script  ──→  Background Worker  ──→  chrome.storage.local
     │                                        ↑
     └──→  Popup UI  ────────────────────────┘
                                              │
                                              ↓  (future)
                                    Rocky Sync API  ──→  ~/.rocky/graph.db
```

## Testing

### Manual Testing

1. Load the extension unpacked in Chrome (see Installation above).
2. Navigate to a YouTube video.
3. Click the Rocky icon — verify URL and title are populated.
4. Open DevTools → Application → Local Storage → verify `pendingResources` updates when you click Add.

### Automated Testing (planned)

```bash
cd extensions/browser
npm install
npm test
```

## Development

### Building from TypeScript

When the build tooling is added:

```bash
npm install
npm run build    # Compiles .ts to .js
npm run watch    # Recompile on changes
```

### Adding New Content Scripts

To support other sites (e.g., MDN, Stack Overflow), add new entries to the `content_scripts` array in `manifest.json` and create corresponding `.ts` files in `src/`.
