import { EVENT_LOCAL_CHANGES } from '@y-sweet/client'
import { YSweetProvider } from '@y-sweet/react'

export async function waitForProviderSync(provider: YSweetProvider, timeoutMillis: number = 1_000) {
  return new Promise<void>((resolve, reject) => {
    provider.on('sync', (synced) => {
      if (synced) {
        resolve()
      }
    })

    setTimeout(() => reject('Timed out waiting for provider to sync.'), timeoutMillis)
  })
}
