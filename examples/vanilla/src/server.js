import express from 'express'
import cors from 'cors'
import { getOrCreateDocAndToken } from '@y-sweet/sdk'

const app = express()
app.use(cors())
app.use(express.json())

const PORT = 9090
const CONNECTION_STRING = process.env.CONNECTION_STRING

if (!CONNECTION_STRING) {
  console.error()
  console.error('CONNECTION_STRING environment variable is required')
  console.error(
    'You can get a connection string at https://app.y-sweet.cloud or by running a y-sweet server locally.',
  )
  console.error()
  console.error(
    'Provide the connection string as an environment variable when starting the server by running:',
  )
  console.error('    CONNECTION_STRING=<your-connection-string> npm run server')
  console.error()
  process.exit(1)
}

app.post('/y-sweet-auth', async (req, res) => {
  const docId = req.body?.docId ?? null
  if (docId) {
    console.log('Received client token request for doc', docId)
  } else {
    console.log('Received client token request for new doc')
  }

  // -------- DO AN AUTH CHECK HERE TO SEE IF THE USER CAN ACCESS THIS DOC --------

  const clientToken = await getOrCreateDocAndToken(CONNECTION_STRING, docId)
  res.send(clientToken)
})

app.listen(PORT, () => {
  console.log(`Y-Sweet VanillaJS demo server listening on port ${PORT}`)
})
