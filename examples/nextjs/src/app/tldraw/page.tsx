import { YDocProvider } from '@y-sweet/react'
import { randomId } from '@/lib/utils'
import Tldraw from './Tldraw'

export default function Home({ searchParams }: { searchParams: { doc: string } }) {
  const docId = searchParams.doc ?? randomId()
  return (
    <YDocProvider docId={docId} setQueryParam="doc" authEndpoint="/api/auth">
      <Tldraw />
    </YDocProvider>
  )
}
