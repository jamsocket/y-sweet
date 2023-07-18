import { HomeInner } from '@/components/HomeInner'
import { createRoom, authDoc } from '../lib/yserv'
import { YjsProvider } from '@/lib/provider'

export default async function Home() {
  let room = await createRoom()
  let auth = await authDoc(room.doc_id, {})

  return (
    <div>
      <h1>Room: {room.doc_id}</h1>
      <h2>Auth: {auth.url}</h2>

      <YjsProvider url={auth.url}>
        <HomeInner />
      </YjsProvider>
    </div>
  )
}

