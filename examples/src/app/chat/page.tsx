import Chat from './Chat'
import { YDocProvider } from '@y-sweet/react'
import { getOrCreateDoc } from '@y-sweet/sdk'

type HomeProps = {
  // searchParams is provided by Next.js. It lets you access the query params in your url.
  // See: https://nextjs.org/docs/app/api-reference/file-conventions/page
  searchParams: Record<string, string>
}

const CONNECTION_STRING =
  'yss://7JyPjmXsHfoUP6DYTLc.AAAgXYAaYMGbNch5VcNW7buoLXxA7W1sC9Dx5UC3K70W-v0@prod.y-sweet.net/p/H4gMt4GME5fq-_WraBQ/'

export default async function Home({ searchParams }: HomeProps) {
  const clientToken = await getOrCreateDoc(searchParams.doc, CONNECTION_STRING)

  return (
    <YDocProvider clientToken={clientToken} setQueryParam="doc">
      <Chat />
    </YDocProvider>
  )
}
