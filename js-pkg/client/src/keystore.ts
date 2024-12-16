import { exportKey, generateEncryptionKey, importKey } from './encryption'

const COOKIE_NAME = 'YSWEET_OFFLINE_KEY'

function getCookie(name: string): string | null {
  const value = `; ${document.cookie}`
  const parts = value.split(`; ${name}=`)
  if (parts.length === 2) return parts.pop()?.split(';').shift() || null
  return null
}

function setCookie(name: string, value: string) {
  document.cookie = `${name}=${value};path=/;expires=Fri, 31 Dec 9999 23:59:59 GMT;secure`
}

export async function getOrCreateKey(): Promise<CryptoKey> {
  // Check for existing key in cookie
  const cookieKey = getCookie(COOKIE_NAME)
  if (cookieKey) {
    // Use existing key from cookie
    return await importKey(cookieKey)
  } else {
    // Generate new key and store in cookie
    const rawKey = await generateEncryptionKey()
    const key = await exportKey(rawKey)
    setCookie(COOKIE_NAME, key)
    return rawKey
  }
}
