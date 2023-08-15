import { Y_SWEET_CONFIG } from '@/lib/config'
import { YDocProvider } from '@y-sweet/react'
import { createDoc, getClientToken, YSweetError } from '@y-sweet/sdk'

type WrappedDocProviderProps = {
  children: React.ReactNode
  searchParams: Record<string, string>
}

function Samp(props: { children: React.ReactNode }) {
  return <samp className="bg-red-100 p-1 rounded-md font-bold">{props.children}</samp>
}

function Error(props: { error: YSweetError }) {
  let { cause } = props.error

  if (cause.code === 'ServerRefused' && cause.address === '127.0.0.1') {
    let portArg = cause.port ? ` --port ${cause.port}` : ''
    return (
      <p>
        It looks like you are trying to connect to a local server, but one isn’t running. Run{' '}
        <Samp>npx y-sweet serve{portArg}</Samp> to start a local server.
      </p>
    )
  } else if (cause.code === 'ServerRefused') {
    return <p>Couldn’t connect to the y-sweet server.</p>
  } else if (cause.code === 'InvalidAuthProvided') {
    return <p>The server token provided was not accepted by the y-sweet server.</p>
  } else if (cause.code === 'NoAuthProvided') {
    return <p>The server expects an authorization header, but the request does not include one.</p>
  } else if (cause.code === 'ServerError') {
    return (
      <>
        <p>
          The server responds, but returned the status code{' '}
          <Samp>
            {cause.status}: {cause.message}
          </Samp>
          .
        </p>
        <p>Make sure this is really a y-sweet server.</p>
      </>
    )
  } else {
    return <p>Check the server console for more information.</p>
  }
}

export async function WrappedDocProvider(props: WrappedDocProviderProps) {
  let docId = props.searchParams.doc

  try {
    if (!docId) {
      let { doc } = await createDoc(Y_SWEET_CONFIG)
      docId = doc
    }

    const clientToken = await getClientToken(docId, {}, Y_SWEET_CONFIG)
    return (
      <YDocProvider clientToken={clientToken} setQueryParam="doc">
        {props.children}
      </YDocProvider>
    )
  } catch (error: any) {
    console.error('Error connecting to y-sweet server', error)

    return (
      <div className="p-10">
        <div className="rounded-md bg-red-50 p-4">
          <div className="flex">
            <div className="ml-3">
              <h3 className="text-sm font-medium text-red-800">
                Error connecting to y-sweet server
              </h3>
              <div className="mt-2 text-sm text-red-700 space-y-2">
                <Error error={error} />
              </div>
            </div>
          </div>
        </div>
      </div>
    )
  }
}
