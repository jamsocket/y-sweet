import { ToDoList } from '@/components/ToDoList'
import { YDocProvider } from '@/lib/provider'
import { getOrCreateDoc } from '../../lib/yserv'

type HomeProps = {
  searchParams: Record<string, string>
}

export default async function Home({ searchParams }: HomeProps) {
  const connectionKey = await getOrCreateDoc(searchParams.ydoc)

  return (
    <YDocProvider connectionKey={connectionKey} setQueryParam='doc'>
      <ToDoList />
    </YDocProvider>
  )
}
