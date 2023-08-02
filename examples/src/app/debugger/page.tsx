import { YDocProvider } from '@y-sweet/react'
import { getClientToken } from '@y-sweet/sdk'
import { Console } from './Console'
import { ENV_CONFIG } from '@/lib/config'

type HomeProps = {
  searchParams: Record<string, string>
}

export default async function Home({ searchParams }: HomeProps) {
  const doc = searchParams.doc

  if (!doc) {
    return <p>Missing doc param</p>
  }

  const clientToken = await getClientToken(searchParams.doc, {}, ENV_CONFIG)

  return (
    <YDocProvider clientToken={clientToken}>
      <Console />
    </YDocProvider>
  )
}
