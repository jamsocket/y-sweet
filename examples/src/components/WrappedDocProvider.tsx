import { Y_SWEET_CONFIG } from "@/lib/config";
import { YDocProvider } from "@y-sweet/react";
import { createDoc, getClientToken } from "@y-sweet/sdk";

type WrappedDocProviderProps = {
    children: React.ReactNode
    searchParams: Record<string, string>
};

function Error(props: { error: any }) {
    let { error } = props

    if (error.cause?.code === 'ECONNREFUSED') {
        let { address, port } = error.cause
        return address === '127.0.0.1' ? // todo: IPV6 support
                <p>It looks like you are trying to connect to a local server, but one isn’t running. Run <samp>npx y-sweet serve{port !== 8080 ? ` --port ${port}` : null}</samp> to run a local server.</p> :
                <p>Try <samp>curl {address}:{port}</samp> to verify the server is running.</p>
    } else if (error.toString().includes("401 Unauthorized")) {
        let containsAuthHeader = false
        if (Y_SWEET_CONFIG) {
            let url = new URL(Y_SWEET_CONFIG)
            containsAuthHeader = url.username !== ''
        }

        return containsAuthHeader ? <p>The server token provided was not accepted by the y-sweet server.</p> : <p>The server expects an authorization header, but the request does not include one.</p>
    } else if (error.toString().includes('Failed to fetch')) {
        return  <p>The server responds, but returns a non-200 status code. Make sure the server is a y-sweet server, and not something else.</p>
    } else {
        return <p>Check the console for more information.</p>
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

        return <div className="p-10">
            <div className="rounded-md bg-red-50 p-4">
                <div className="flex">
                    <div className="ml-3">
                        <h3 className="text-sm font-medium text-red-800">Error connecting to y-sweet server</h3>
                        <div className="mt-2 text-sm text-red-700">
                            <Error error={error} />
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}
