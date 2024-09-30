import { YDocProvider } from '@y-sweet/react'
import { randomId } from '@/lib/utils'
import { Presence } from './Presence'

export default function Home({ searchParams }: { searchParams: { doc: string } }) {
  const docId = searchParams.doc ?? randomId()
  return (
    <YDocProvider docId={docId} setQueryParam="doc" authEndpoint="/api/auth">
      <Presence />
    </YDocProvider>
  )
}
