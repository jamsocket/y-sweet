import { ToDoList } from '@/components/ToDoList'
import { YjsProvider } from '@/lib/provider'
import { getConnectionKey, createRoom } from '../lib/yserv'

type HomeProps = {
  searchParams: Record<string, string>
}

export default async function Home({searchParams}: HomeProps) {
  console.log(searchParams)
  let ydoc = searchParams.ydoc

  if (!ydoc) {
    let room = await createRoom()
    ydoc = room.doc_id
  }

  let connectionKey = await getConnectionKey(ydoc, {})

  return (
    <YjsProvider connectionKey={connectionKey} setQueryParam='ydoc'>
      <ToDoList />
    </YjsProvider>
  )
}
