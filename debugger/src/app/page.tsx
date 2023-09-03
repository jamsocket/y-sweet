"use client"

import { YDocProvider } from "@y-sweet/react"
import { ClientToken, decodeClientToken } from "@y-sweet/sdk"
import { useEffect, useState } from "react"
import { Console } from "./Console"

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
    return <div>
      Debug payload: {JSON.stringify(clientToken)}

      <Debugger clientToken={clientToken} />
    </div>
  } else {
    return <div>Y-Sweet Debugger.</div>
  }
}

function Debugger(props: {clientToken: ClientToken}) {
  return <YDocProvider clientToken={props.clientToken}>
    <Console />
  </YDocProvider>
}
