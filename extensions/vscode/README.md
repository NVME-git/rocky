# Rocky VS Code Extension

View and explore your [Rocky](https://github.com/NVME-git/rocky) personal knowledge graph (PKG) directly in VS Code.

## Features

| Feature | Description |
|---------|-------------|
| **Topics Tree** | Browse all tracked topics grouped by domain (Language, Auth, DevOps, …) with retrievability indicators |
| **Summary Panel** | At-a-glance statistics — total topics, connections, average retrievability, weak/strong counts |
| **Interactive Graph** | Force-directed knowledge graph rendered in a webview, with drag, hover tooltips, and domain colouring |
| **Global & Local Scope** | Switch between your full PKG and only the topics linked to the current workspace repository |
| **Topic Detail** | Click any topic to see difficulty, stability, retrievability bar, review count, and canonical Q&A |

## Prerequisites

- **Rocky CLI** installed and on your `PATH` (or configure the path in settings)  
  → Install with `cargo install --path .` from the Rocky repo root
- A populated knowledge graph — run `rocky backup` at least once so the extension can read `~/.rocky/pkg/pkg.json`

## Installation

### From Source (development)

```bash
cd extensions/vscode
npm install
npm run compile
```

Then press **F5** in VS Code to launch the Extension Development Host.

### From VSIX (packaged)

```bash
cd extensions/vscode
npm install
npx vsce package        # produces rocky-vscode-0.1.0.vsix
code --install-extension rocky-vscode-0.1.0.vsix
```

## Configuration

Open **Settings → Extensions → Rocky** or add to your `settings.json`:

```jsonc
{
  // Path to the global Rocky directory (default: ~/.rocky)
  "rocky.globalDbPath": "",

  // Path to the Rocky CLI binary (default: "rocky")
  "rocky.cliPath": "rocky"
}
```

## Usage

### Topics Tree

1. Open the **Rocky** activity bar icon (left sidebar).
2. Expand any domain group to see individual topics.
3. Topics are colour-coded by retrievability:
   - 🟢 **>70 %** — strong recall
   - 🟡 **40–70 %** — fading
   - 🔴 **<40 %** — needs review
4. Click a topic to open the detail panel.

### Interactive Graph

- **Command Palette → Rocky: Open Knowledge Graph** (or the graph icon in the Topics view toolbar)
- **Rocky: Open Global Knowledge Graph** — shows all topics
- **Rocky: Open Local Knowledge Graph** — shows only topics tagged with the current workspace repo

Nodes are sized by review count and coloured by domain. Hover for details, drag to rearrange.

### Refreshing Data

Click the refresh icon in the Topics view title bar, or run **Rocky: Refresh Topics** from the Command Palette. This re-reads `pkg.json` from disk.

## Architecture

```
extensions/vscode/
├── src/
│   ├── extension.ts          # Activation, command registration
│   ├── topicsTreeProvider.ts # Tree view: topics grouped by domain
│   ├── summaryTreeProvider.ts# Tree view: PKG statistics
│   ├── graphPanel.ts         # Webview: force-directed graph
│   ├── rockyDataProvider.ts  # Reads & caches pkg.json
│   └── test/
│       ├── extension.test.ts # Unit tests
│       └── runTest.ts        # Mocha test runner
├── media/
│   └── rocky-icon.svg        # Activity bar icon
├── package.json              # Extension manifest & contributes
├── tsconfig.json             # TypeScript config
└── README.md                 # This file
```

### Data Flow

```
~/.rocky/pkg/pkg.json  ──→  RockyDataProvider  ──→  TopicsTreeProvider
                                                 ──→  SummaryTreeProvider
                                                 ──→  GraphPanel (webview)
```

The extension reads the JSON backup that Rocky produces with `rocky backup`. It does **not** access the SQLite database directly, keeping the dependency footprint minimal.

## Testing

```bash
cd extensions/vscode
npm install
npm run compile
npm test
```

Unit tests cover data transformation logic (grouping, filtering, statistics, HTML escaping) without requiring the VS Code API runtime.

## Development

### Watch mode

```bash
npm run watch
```

Then press **F5** to launch the Extension Development Host. Changes recompile automatically.

### Linting

```bash
npm run lint
```

## Commands Reference

| Command | ID | Description |
|---------|----|-------------|
| Refresh Topics | `rocky.refreshTopics` | Re-read pkg.json from disk |
| Open Knowledge Graph | `rocky.openGraph` | Open the interactive global graph |
| Open Global Knowledge Graph | `rocky.openGlobalGraph` | Open the interactive global graph |
| Open Local Knowledge Graph | `rocky.openLocalGraph` | Open the graph filtered to current repo |
| Show Topic Detail | `rocky.showTopicDetail` | Open detail panel for a topic |
