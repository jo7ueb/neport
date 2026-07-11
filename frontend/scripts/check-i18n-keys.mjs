import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import path from 'node:path'

const dir = path.dirname(fileURLToPath(import.meta.url))
const en = JSON.parse(readFileSync(path.join(dir, '../src/i18n/locales/en.json'), 'utf-8'))
const ja = JSON.parse(readFileSync(path.join(dir, '../src/i18n/locales/ja.json'), 'utf-8'))

function flattenKeys(obj, prefix = '') {
  const keys = []
  for (const [k, v] of Object.entries(obj)) {
    const fullKey = prefix ? `${prefix}.${k}` : k
    if (v && typeof v === 'object' && !Array.isArray(v)) {
      keys.push(...flattenKeys(v, fullKey))
    } else {
      keys.push(fullKey)
    }
  }
  return keys
}

const enKeys = new Set(flattenKeys(en))
const jaKeys = new Set(flattenKeys(ja))

const missingInJa = [...enKeys].filter((k) => !jaKeys.has(k))
const missingInEn = [...jaKeys].filter((k) => !enKeys.has(k))

if (missingInJa.length > 0 || missingInEn.length > 0) {
  if (missingInJa.length > 0) {
    console.error('Keys present in en.json but missing in ja.json:')
    for (const k of missingInJa) console.error(`  - ${k}`)
  }
  if (missingInEn.length > 0) {
    console.error('Keys present in ja.json but missing in en.json:')
    for (const k of missingInEn) console.error(`  - ${k}`)
  }
  process.exit(1)
}

console.log(`i18n key check passed (${enKeys.size} keys, en/ja in sync).`)
