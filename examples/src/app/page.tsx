import { ToDoList } from '@/components/ToDoList'
import { YjsProvider } from '@/lib/provider'
import { getOrCreateDoc } from '../lib/yserv'

type HomeProps = {
  searchParams: Record<string, string>
}

export default async function Home({ searchParams }: HomeProps) {
  const connectionKey = await getOrCreateDoc(searchParams.ydoc)

  return (
    <YjsProvider connectionKey={connectionKey} setQueryParam='ydoc'>
      <ToDoList />
    </YjsProvider>
  )
}
