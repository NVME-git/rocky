/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{astro,html,js,jsx,ts,tsx,md,mdx}'],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        // Brand
        brand: {
          DEFAULT: '#F59E0B',
          dim: '#D97706',
        },
        accent: {
          DEFAULT: '#06B6D4',
          dim: '#0891B2',
        },

        // Semantic recall buckets (mirror the CLI truecolors)
        known: '#1D9E75',
        fading: '#EF9F27',
        gap: '#E24B4A',

        // Surfaces
        bg: {
          DEFAULT: '#0A0E1A',     // dark page bg
          light: '#F8FAFC',
        },
        surface: {
          DEFAULT: '#111827',
          light: '#FFFFFF',
        },
        sidebar: {
          DEFAULT: '#070B14',
          hover: '#0F1629',
          light: '#F1F5F9',
          'hover-light': '#E2E8F0',
        },
        terminal: {
          bg: '#0d1117',          // terminal block bg (always dark)
          header: '#161b22',
          border: '#30363d',
        },
        divider: {
          DEFAULT: '#1E293B',
          light: '#E2E8F0',
        },
        card: {
          DEFAULT: '#0F172A',
          light: '#F8FAFC',
        },

        // Text
        ink: {
          DEFAULT: '#F1F5F9',     // textPrimary dark
          light: '#0F172A',       // textPrimary light
          mute: '#CBD5E1',        // textSecondary dark
          'mute-light': '#334155',
          dim: '#64748B',         // textMuted dark
          'dim-light': '#94A3B8',
        },
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', '-apple-system', 'Segoe UI', 'Roboto', 'sans-serif'],
        mono: ['JetBrains Mono', 'Menlo', 'Monaco', 'Consolas', 'monospace'],
      },
    },
  },
  plugins: [],
};
