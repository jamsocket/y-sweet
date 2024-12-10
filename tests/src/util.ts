import { YSweetProvider } from '@y-sweet/react'

export async function waitForProviderSync(provider: YSweetProvider, timeoutMillis: number = 1_000) {
  return new Promise((resolve, reject) => {
    provider.on('sync', resolve)

    setTimeout(() => reject('Timed out waiting for provider to sync.'), timeoutMillis)
  })
}
