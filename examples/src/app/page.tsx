import { ToDoList } from '@/components/ToDoList'
import { YjsProvider } from '@/lib/provider'
import { authDoc, createRoom } from '../lib/yserv'

export default async function Home() {
  let room = await createRoom()
  let auth = await authDoc(room.doc_id, {})

  return (
    <YjsProvider base_url={auth.base_url} doc_id={auth.doc_id}>
      <ToDoList />
    </YjsProvider>
  )
}
