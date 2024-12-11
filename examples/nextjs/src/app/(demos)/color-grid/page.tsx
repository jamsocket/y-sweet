import { YDocProvider } from '@y-sweet/react'
import { randomId } from '@/lib/utils'
import { ColorGrid } from './ColorGrid'
import StateIndicator from '@/components/StateIndicator'

export default function Home({ searchParams }: { searchParams: { doc: string } }) {
  const docId = searchParams.doc ?? randomId()
  return (
<<<<<<< HEAD
    <YDocProvider docId={docId} setQueryParam="doc" authEndpoint="/api/auth">
      <div className="p-4 lg:p-8">
        <StateIndicator />
        <ColorGrid />
      </div>
=======
    <YDocProvider docId={docId} setQueryParam="doc" authEndpoint="/api/auth" enableOfflineSupport={true}>
      <ColorGrid />
>>>>>>> 59135b9 (initial offline support)
    </YDocProvider>
  )
}
