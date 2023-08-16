'use client'

function Samp(props: { children: React.ReactNode }) {
  return <samp className="bg-red-100 p-1 rounded-md font-bold">{props.children}</samp>
}

export default function Error(props: { error: any }) {
  let message = <p>Check the server console for more information.</p>

  const errMsg = props.error.message
  if (errMsg.startsWith('ServerRefused') && errMsg.includes('127.0.0.1')) {
    const port = /127\.0\.0\.1:(\d+)/.exec(errMsg)?.[1]
    const portArg = port ? ` --port ${port}` : ''
    message = (
      <p>
        It looks like you are trying to connect to a local server, but one isn’t running. Run{' '}
        <Samp>npx y-sweet serve{portArg}</Samp> to start a local server.
      </p>
    )
  } else if (errMsg.startsWith('ServerRefused')) {
    message = <p>Couldn’t connect to the y-sweet server.</p>
  } else if (errMsg.startsWith('InvalidAuthProvided')) {
    message = <p>The server token provided was not accepted by the y-sweet server.</p>
  } else if (errMsg.startsWith('NoAuthProvided')) {
    message = (
      <p>The server expects an authorization header, but the request does not include one.</p>
    )
  } else if (errMsg.startsWith('ServerError')) {
    message = (
      <>
        <p>
          The server responds, but returned: <Samp>{errMsg}</Samp>.
        </p>
        <p>Make sure this is really a y-sweet server.</p>
      </>
    )
  }

  return (
    <div className="p-10">
      <div className="rounded-md bg-red-50 px-6 py-4 text-md text-red-700 space-y-2">
        <h3 className="font-medium text-red-800">Error connecting to y-sweet server</h3>
        {message}
      </div>
    </div>
  )
}
