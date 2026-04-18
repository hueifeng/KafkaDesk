/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{html,js,jsx,ts,tsx}'],
  theme: {
    extend: {
      colors: {
        app: 'hsl(var(--bg-app) / <alpha-value>)',
        surface: 'hsl(var(--bg-surface) / <alpha-value>)',
        panel: 'hsl(var(--bg-panel) / <alpha-value>)',
        elevated: 'hsl(var(--bg-elevated) / <alpha-value>)',
        line: 'hsl(var(--border-subtle) / <alpha-value>)',
        ink: 'hsl(var(--text-primary) / <alpha-value>)',
        'ink-muted': 'hsl(var(--text-muted) / <alpha-value>)',
        'ink-dim': 'hsl(var(--text-secondary) / <alpha-value>)',
        signal: 'hsl(var(--accent-inspect) / <alpha-value>)',
        warning: 'hsl(var(--accent-warning) / <alpha-value>)',
        danger: 'hsl(var(--accent-danger) / <alpha-value>)',
        success: 'hsl(var(--accent-success) / <alpha-value>)',
      },
      fontSize: {
        body: ['0.95rem', { lineHeight: '1.5rem' }],
        table: ['0.84rem', { lineHeight: '1.25rem' }],
      },
      boxShadow: {
        panel: 'var(--shadow-panel)',
        chrome: 'var(--shadow-chrome)',
      },
    },
  },
  plugins: [],
};
