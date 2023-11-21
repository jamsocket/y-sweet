'use client'

import { YSweetError } from '@y-sweet/sdk'

function Samp(props: { children: React.ReactNode }) {
  return <samp className="bg-red-100 p-1 rounded-md font-bold">{props.children}</samp>
}

function ErrorMessage(props: { error: YSweetError }) {
  let { cause } = props.error

  if (!(props.error instanceof YSweetError)) {
    throw props.error
  }

  if (cause.code === 'ServerRefused' && cause.address === '127.0.0.1') {
    let portArg = cause.port === 8080 ? '' : ` --port ${cause.port}`
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
    return <p>Check the console for more information.</p>
  }
}

export default function Error(props: { error: any }) {
  let message = <p>Check the server console for more information.</p>

  const errMsg = props.error.message
  const error = YSweetError.fromMessage(errMsg)

  return (
    <div className="p-10">
      <div className="rounded-md bg-red-50 px-6 py-4 text-md text-red-700 space-y-2">
        <h3 className="font-medium text-red-800">Error connecting to y-sweet server</h3>
        <ErrorMessage error={error} />
      </div>
    </div>
  )
}
