import { CONNECTION_STRING } from '@/lib/config'
import { getOrCreateDocAndToken } from '@y-sweet/sdk'
import { NextResponse } from 'next/server'

const CORS_HEADERS = {
  'access-control-allow-origin': '*',
  'access-control-allow-methods': 'POST',
  'access-control-allow-headers': 'content-type',
}

export async function OPTIONS(request: Request) {
  return new Response(null, { status: 204, headers: CORS_HEADERS })
}

export async function POST(request: Request) {
  // In a production app, you'd want to validate that the user is authenticated
  // and has access to the given doc.

  const { docId } = await request.json()
  const clientToken = await getOrCreateDocAndToken(CONNECTION_STRING, docId)
  return NextResponse.json(clientToken, { headers: CORS_HEADERS })
}
