import { ToDoList } from './ToDoList'
import { YDocProvider } from '@/lib/provider'
import { getOrCreateDoc } from '@/lib/yserv'

type HomeProps = {
  searchParams: Record<string, string>
}

export default async function Home({ searchParams }: HomeProps) {
  const connectionKey = await getOrCreateDoc(searchParams.doc, {token: 'QUFBQUFBQWdBQUFBQUFBQUFKUGovMG5iV2dGS25XSUFiOUt0Z0tlMW9GaTAvZ2dKMXJmbWFyenlIVmZ5'})

  return (
    <YDocProvider connectionKey={connectionKey} setQueryParam='doc'>
      <ToDoList />
    </YDocProvider>
  )
}
