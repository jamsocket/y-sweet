import { ToDoList } from '@/components/ToDoList'
import { YjsProvider } from '@/lib/provider'
import { authDoc, createRoom } from '../lib/yserv'

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

  let auth = await authDoc(ydoc, {})

  return (
    <YjsProvider baseUrl={auth.base_url} docId={auth.doc_id} setQueryParam='ydoc'>
      <ToDoList />
    </YjsProvider>
  )
}
