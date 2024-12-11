function arrayBufferToBase64(buffer: ArrayBuffer): string {
  const bytes = new Uint8Array(buffer)
  let binary = ''
  for (let i = 0; i < bytes.byteLength; i++) {
    binary += String.fromCharCode(bytes[i])
  }
  return btoa(binary)
}

function base64ToArrayBuffer(base64: string): ArrayBuffer {
  const binary = atob(base64)
  const bytes = new Uint8Array(binary.length)
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i)
  }
  return bytes.buffer
}

export async function importKey(key: string): Promise<CryptoKey> {
  const rawKey = base64ToArrayBuffer(key)
  return await crypto.subtle.importKey('raw', rawKey, { name: 'AES-GCM', length: 256 }, true, [
    'encrypt',
    'decrypt',
  ])
}

export async function exportKey(key: CryptoKey): Promise<string> {
  const rawKey = await crypto.subtle.exportKey('raw', key)
  return arrayBufferToBase64(new Uint8Array(rawKey))
}

export function generateEncryptionKey(): Promise<CryptoKey> {
  return crypto.subtle.generateKey({ name: 'AES-GCM', length: 256 }, true, ['encrypt', 'decrypt'])
}

export async function encryptData(data: Uint8Array, key: CryptoKey): Promise<Uint8Array> {
  const iv = crypto.getRandomValues(new Uint8Array(12))
  const encrypted = await crypto.subtle.encrypt({ name: 'AES-GCM', iv }, key, data)

  const result = new Uint8Array(iv.length + encrypted.byteLength)
  result.set(iv)
  result.set(new Uint8Array(encrypted), iv.length)
  return result
}

export async function decryptData(encryptedData: Uint8Array, key: CryptoKey): Promise<Uint8Array> {
  const iv = encryptedData.slice(0, 12)
  const data = encryptedData.slice(12)
  const decrypted = await crypto.subtle.decrypt({ name: 'AES-GCM', iv }, key, data)
  return new Uint8Array(decrypted)
}
