import typography from '@tailwindcss/typography'

/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  theme: {
    extend: {
      colors: {
        // ブランドキーカラー #333399 を基準にした配色（blue-700 が基準色）
        blue: {
          50: '#f4f4fb',
          100: '#e4e4f6',
          200: '#c6c6ec',
          300: '#9c9cde',
          400: '#6666cc',
          500: '#3d3db8',
          600: '#3838a8',
          700: '#333399',
          800: '#29297a',
          900: '#1f1f5c',
          950: '#131339',
        },
      },
    },
  },
  plugins: [typography],
}
