import { YSweetProvider } from '@y-sweet/react'

export async function waitForProviderSync(provider: YSweetProvider, timeoutMillis: number = 1_000) {
  return new Promise<void>((resolve, reject) => {
    function onSync(synced: boolean) {
      if (synced) {
        resolve()
      }
    }
    provider.once('sync', onSync)
    setTimeout(() => {
      provider.off('sync', onSync)
      reject('Timed out waiting for provider to sync.')
    }, timeoutMillis)
  })
}

export async function waitForProviderSyncChanges(
  provider: YSweetProvider,
  timeoutMillis: number = 1_000,
) {
  return new Promise<void>((resolve, reject) => {
    // wait for local changes to be false, meaning that the provider has synced its changes to the server
    function onLocalChanges(hasLocalChanges: boolean) {
      if (!hasLocalChanges) {
        resolve()
      }
    }
    provider.on('local-changes', onLocalChanges)
    setTimeout(() => {
      provider.off('local-changes', onLocalChanges)
      reject('Timed out waiting for provider to have local changes.')
    }, timeoutMillis)
  })
}
