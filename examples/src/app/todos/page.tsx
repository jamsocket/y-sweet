import { ToDoList } from './ToDoList'
import { YDocProvider } from '@/lib/provider'
import { getOrCreateDoc } from '@/lib/yserv'

type HomeProps = {
  searchParams: Record<string, string>
}

export default async function Home({ searchParams }: HomeProps) {
  const connectionKey = await getOrCreateDoc(searchParams.doc, {token: 'a57c50b35bc1ce2e2f6eb25a270674072ecefafa46b1bd6696b448544439acb5'})

  return (
    <YDocProvider connectionKey={connectionKey} setQueryParam='doc'>
      <ToDoList />
    </YDocProvider>
  )
}
