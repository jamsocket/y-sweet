import { YSweetProvider } from '@y-sweet/react'

export async function waitForProviderSync(provider: YSweetProvider, timeoutMillis: number = 1_000) {
  return new Promise((resolve, reject) => {
    provider.on('synced', resolve)
    provider.on('syncing', reject)

    setTimeout(() => reject('Timed out waiting for provider to sync.'), timeoutMillis)
  })
}
