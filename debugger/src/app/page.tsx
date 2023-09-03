'use client'

import { YDocProvider } from '@y-sweet/react'
import { ClientToken, decodeClientToken } from '@y-sweet/sdk'
import { useEffect, useState } from 'react'
import { Debugger } from './debugger'

export default function Home() {
  const [clientToken, setClientToken] = useState<ClientToken | null>(null)

  useEffect(() => {
    const urlParams = new URLSearchParams(window.location.search)
    const payload = urlParams.get('payload')

    if (payload) {
      setClientToken(decodeClientToken(payload))
    }
  }, [])

  if (clientToken) {
    console.log('clientToken', clientToken)

    return (
      <div>
        <DebuggerWrapper clientToken={clientToken} />
      </div>
    )
  } else {
    return <div>Y-Sweet Debugger.</div>
  }
}

function DebuggerWrapper(props: { clientToken: ClientToken }) {
  return (
    <YDocProvider clientToken={props.clientToken}>
      <Debugger />
    </YDocProvider>
  )
}
