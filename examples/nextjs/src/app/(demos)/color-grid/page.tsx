import { YDocProvider } from '@y-sweet/react'
import { randomId } from '@/lib/utils'
import { ColorGrid } from './ColorGrid'
export default function Home({ searchParams }: { searchParams: { doc: string } }) {
  const docId = searchParams.doc ?? randomId()
  return (
    <YDocProvider docId={docId} setQueryParam="doc" authEndpoint="/api/auth" offlineSupport={true}>
      <div className="p-4 lg:p-8">
        <ColorGrid />
      </div>
    </YDocProvider>
  )
}
