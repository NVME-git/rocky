// Single source of truth for the sidebar order + slugs + icons.
// Mirrors the kSections list from the old Flutter app.

export interface Section {
  slug: string;
  title: string;
  icon: string; // inline SVG path data (Material-style 24x24)
}

// Material Symbols (Outlined) path data — picked to match the old Flutter
// IconData choices as closely as possible.
const I = {
  home: 'M12 3l9 8h-2v9h-5v-6h-4v6H5v-9H3l9-8z',
  download: 'M5 20h14v-2H5v2zM12 3v10.17l3.59-3.58L17 11l-5 5-5-5 1.41-1.41L11 13.17V3h2z',
  rocket: 'M9.19 6.35a4.5 4.5 0 016.36 6.36l-1.41 1.41-6.36-6.36 1.41-1.41zM6.34 9.19l6.36 6.36-1.41 1.41a4.5 4.5 0 01-6.36-6.36l1.41-1.41zm-1.06 11.32l1.41-1.41 1.41 1.41-1.41 1.41-1.41-1.41z',
  terminal: 'M2 5h20v14H2V5zm2 2v10h16V7H4zm2 2l4 3-4 3v-2l1.5-1L6 11V9zm6 4h6v2h-6v-2z',
  settings: 'M19.43 12.98c.04-.32.07-.64.07-.98s-.03-.66-.07-.98l2.11-1.65a.5.5 0 00.12-.64l-2-3.46a.5.5 0 00-.61-.22l-2.49 1a7.32 7.32 0 00-1.69-.98l-.38-2.65A.5.5 0 0014 2h-4a.5.5 0 00-.5.42l-.38 2.65a7.32 7.32 0 00-1.69.98l-2.49-1a.5.5 0 00-.61.22l-2 3.46a.5.5 0 00.12.64l2.11 1.65c-.04.32-.07.65-.07.98s.03.66.07.98l-2.11 1.65a.5.5 0 00-.12.64l2 3.46a.5.5 0 00.61.22l2.49-1c.5.39 1.07.71 1.69.98l.38 2.65A.5.5 0 0010 22h4a.5.5 0 00.5-.42l.38-2.65a7.32 7.32 0 001.69-.98l2.49 1a.5.5 0 00.61-.22l2-3.46a.5.5 0 00-.12-.64l-2.11-1.65zM12 15.5a3.5 3.5 0 110-7 3.5 3.5 0 010 7z',
  tree: 'M22 11V3h-7v3H9V3H2v8h7V8h2v10h4v3h7v-8h-7v3h-2V8h2v3h7zM7 9H4V5h3v4zm10 6h3v4h-3v-4zm0-10h3v4h-3V5z',
  sync: 'M12 4V1L8 5l4 4V6c3.31 0 6 2.69 6 6 0 1.01-.25 1.97-.7 2.8l1.46 1.46A7.93 7.93 0 0020 12c0-4.42-3.58-8-8-8zm0 14c-3.31 0-6-2.69-6-6 0-1.01.25-1.97.7-2.8L5.24 7.74A7.93 7.93 0 004 12c0 4.42 3.58 8 8 8v3l4-4-4-4v3z',
  hub: 'M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5',
  play: 'M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 14.5v-9l6 4.5-6 4.5z',
  mic: 'M12 14a3 3 0 003-3V5a3 3 0 00-6 0v6a3 3 0 003 3zm5-3a5 5 0 01-10 0H5a7 7 0 006 6.92V21h2v-3.08A7 7 0 0019 11h-2z',
  book: 'M21 5c-1.11-.35-2.33-.5-3.5-.5-1.95 0-4.05.4-5.5 1.5-1.45-1.1-3.55-1.5-5.5-1.5S2.45 4.9 1 6v14.65c0 .25.25.5.5.5.1 0 .15-.05.25-.05C3.1 20.45 5.05 20 6.5 20c1.95 0 4.05.4 5.5 1.5 1.35-.85 3.8-1.5 5.5-1.5 1.65 0 3.35.3 4.75 1.05.1.05.15.05.25.05.25 0 .5-.25.5-.5V6c-.6-.45-1.25-.75-2-1zm0 13.5c-1.1-.35-2.3-.5-3.5-.5-1.7 0-4.15.65-5.5 1.5V8c1.35-.85 3.8-1.5 5.5-1.5 1.2 0 2.4.15 3.5.5v11.5z',
};

export const sections: Section[] = [
  { slug: 'introduction',      title: 'What is Rocky?', icon: I.home },
  { slug: 'installation',      title: 'Installation',   icon: I.download },
  { slug: 'quick-start',       title: 'Quick Start',    icon: I.rocket },
  { slug: 'commands',          title: 'Commands',       icon: I.terminal },
  { slug: 'configuration',     title: 'Configuration',  icon: I.settings },
  { slug: 'how-it-works',      title: 'How It Works',   icon: I.tree },
  { slug: 'sync-and-backup',   title: 'Sync & Backup',  icon: I.sync },
  { slug: 'obsidian',          title: 'Obsidian',       icon: I.hub },
  { slug: 'demo',              title: 'Demo',           icon: I.play },
  { slug: 'voice',             title: 'Voice',          icon: I.mic },
  { slug: 'references',        title: 'References',     icon: I.book },
];
