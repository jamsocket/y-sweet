import { ToDoList } from '@/components/ToDoList'
import { createRoom, authDoc } from '../lib/yserv'
import { YjsProvider } from '@/lib/provider'

export default async function Home() {
  let room = await createRoom()
  let auth = await authDoc(room.doc_id, {})

  return (
    <YjsProvider base_url={auth.base_url} doc_id={auth.doc_id}>
      <ToDoList />
    </YjsProvider>
  )
}

