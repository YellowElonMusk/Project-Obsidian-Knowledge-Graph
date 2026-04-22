/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  theme: {
    extend: {
      colors: {
        bg: '#0a0a0f',
        surface: '#12121e',
        panel: '#1a1a2e',
        border: '#2d2d4e',
        accent: '#7c3aed',
        'accent-light': '#a855f7',
        'accent-dim': '#4c1d95',
        text: '#e2e8f0',
        muted: '#64748b',
        'node-file': '#3b82f6',
        'node-concept': '#8b5cf6',
        'node-person': '#10b981',
        'node-task': '#f59e0b',
        'node-decision': '#ef4444',
        'node-session': '#6366f1',
        'node-code': '#06b6d4',
        'node-agent': '#ec4899',
      },
      fontFamily: {
        mono: ['JetBrains Mono', 'Fira Code', 'ui-monospace', 'monospace'],
      },
      animation: {
        'pulse-slow': 'pulse 3s cubic-bezier(0.4, 0, 0.6, 1) infinite',
        'spin-slow': 'spin 4s linear infinite',
        'glow': 'glow 2s ease-in-out infinite alternate',
        'float': 'float 3s ease-in-out infinite',
      },
      keyframes: {
        glow: {
          '0%': { boxShadow: '0 0 5px #7c3aed44, 0 0 10px #7c3aed22' },
          '100%': { boxShadow: '0 0 20px #7c3aed88, 0 0 40px #7c3aed44' },
        },
        float: {
          '0%, 100%': { transform: 'translateY(0)' },
          '50%': { transform: 'translateY(-4px)' },
        },
      },
      backdropBlur: {
        xs: '2px',
      },
    },
  },
  plugins: [],
}
