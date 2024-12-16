const ALGORITHM: AesKeyGenParams = {
  name: 'AES-GCM',
  length: 256,
}
const KEY_USAGE: KeyUsage[] = ['encrypt', 'decrypt']

const NONCE_LENGTH = 12

function arrayBufferToBase64(buffer: ArrayBuffer): string {
  return btoa(String.fromCharCode(...new Uint8Array(buffer)))
}

function base64ToArrayBuffer(base64: string): ArrayBuffer {
  const binaryString = atob(base64)
  const bytes = Uint8Array.from(binaryString, (c) => c.charCodeAt(0))
  return bytes.buffer
}

export async function importKey(key: string): Promise<CryptoKey> {
  const rawKey = base64ToArrayBuffer(key)
  return await crypto.subtle.importKey('raw', rawKey, ALGORITHM, true, KEY_USAGE)
}

export async function exportKey(key: CryptoKey): Promise<string> {
  const rawKey = await crypto.subtle.exportKey('raw', key)
  return arrayBufferToBase64(rawKey)
}

export function generateEncryptionKey(): Promise<CryptoKey> {
  return crypto.subtle.generateKey(ALGORITHM, true, KEY_USAGE)
}

export async function encryptData(data: Uint8Array, key: CryptoKey): Promise<Uint8Array> {
  const iv = crypto.getRandomValues(new Uint8Array(NONCE_LENGTH))
  const encrypted = await crypto.subtle.encrypt({ name: ALGORITHM.name, iv }, key, data)

  const result = new Uint8Array(iv.length + encrypted.byteLength)
  result.set(iv)
  result.set(new Uint8Array(encrypted), iv.length)
  return result
}

export async function decryptData(encryptedData: Uint8Array, key: CryptoKey): Promise<Uint8Array> {
  const iv = encryptedData.slice(0, NONCE_LENGTH)
  const data = encryptedData.slice(NONCE_LENGTH)
  const decrypted = await crypto.subtle.decrypt({ name: ALGORITHM.name, iv }, key, data)
  return new Uint8Array(decrypted)
}
