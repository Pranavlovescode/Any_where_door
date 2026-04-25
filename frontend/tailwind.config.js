/** @type {import('tailwindcss').Config} */
export default {
  darkMode: 'class',
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      colors: {
        ink: {
          50: '#f4f8ff',
          950: '#080f1f',
        },
      },
      fontFamily: {
        display: ['Sora', 'ui-sans-serif', 'system-ui'],
        sans: ['Space Grotesk', 'ui-sans-serif', 'system-ui'],
      },
      boxShadow: {
        glow: '0 20px 80px rgba(0, 234, 255, 0.15)',
      },
      keyframes: {
        drift: {
          '0%, 100%': { transform: 'translate3d(0, 0, 0)' },
          '50%': { transform: 'translate3d(0, -14px, 0)' },
        },
      },
      animation: {
        drift: 'drift 7s ease-in-out infinite',
      },
    },
  },
  plugins: [],
}

